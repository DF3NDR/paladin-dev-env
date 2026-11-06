# Phase 55 base-layer review (content / general data orchestration items)

Plan 55-12, requirement FIXES-02, decisions D-03 and D-00e. This file records the keep / fence decision
for the content and general-data items that the 2026-10-13 todo and ADR-0060 (g) left unreviewed. It is
the record the rustdoc on the retained items and on both `ContentListRepository` impls points at.

- **Date:** 2026-11-01
- **Head SHA read:** `b51e9ca2e3b2e939e52a5ee2d48305514ccead5c` (`b51e9ca2e`), branch `claude/laughing-dirac-e0h2ax`.
  Every `file:line` below was read at this head by the executor, not copied from the plan summaries; the
  summaries (55-01, 55-05, 55-06) supplied the caller reads, which were re-checked by a fresh `grep` (see
  "Caller re-check").

**Verdict: every item is kept. Five items that invented a value or reported a success they did not earn are
fenced (a typed refusal or a documented sentinel); nothing is removed, renamed or finished in this milestone.
Finishing or retiring the system is a next-minor decision (see "Next-minor candidates").**

## Scope

Eight items, one row each: `Field<T>`; `CollectionType<T>` / `Item<T>`; the comment-only `element` module;
`FieldVersionService<T>`; `CollectionVersionService<T>`; `planning_service.rs`'s placeholder expected output;
`file_content_fetcher.rs`'s `ingest_content` and its three metadata helpers; and the twenty
`ContentListRepository` bodies (ten on `SqliteStore`, ten on `MySqlContentRepository`). The first five are
the base items of the todo; the last three are the stub-shaped content / planning bodies the phase research
inventoried (G3 and the planner placeholder).

## Inputs

- The todo `2026-10-13-review-content-data-orchestration-base-items.md` (problem statement, options A, B and C,
  the owner position of 2026-10-13, the residual entity-sense identifiers).
- `crates/paladin-core/src/base/mod.rs` "Retained pending review" (as updated by plan 55-05) and ADR-0060 (g)
  (`.planning/decisions/0060-base-entity-rename.md`, section "(g) The base-layer disposition").
- `.project/Milestone_1-MVP`: read. It holds ten Epics (`Epic_1` to `Epic_10`) of Paladin, Battalion, Garrison,
  Arsenal, Citadel and CLI work. A case-insensitive search for `content list`, `general data`,
  `FieldVersion`, `CollectionVersion` and `ContentListRepository` over the directory returns nothing: it
  carries no content or general-data design.
- Plan summaries `55-01-SUMMARY.md` (storage refusal shape), `55-05-SUMMARY.md` (per-item design intent and
  current state), `55-06-SUMMARY.md` (the `ingest_content` caller read and the `expected_output` choice).

**Finding: no design record exists beyond the todo's own account.** The only statement of what the content / general data
orchestration system was meant to be is the owner's answer of 2026-10-13 quoted in the todo ("started with the
original base layer and never finished"). No PRD, ADR, task list or milestone file describes it, so the
"design intent" column below is the intent each item's own code and comments express, not a recovered design.

## Decisions

| Item | Design intent | Current state | Decision | Evidence |
| ---- | ---- | ---- | ---- | ---- |
| `Field<T>` (`fid`, `nid`, `value`) | One value held as a child of an `Entity` (username, email), the Drupal-style field; a base type for specialised fields. | Plain struct, 36 lines. `Field::new` assigns a fresh `nid` that refers to no parent. Only consumer is `FieldVersionService`. Rustdoc says unfinished and names this review. | Keep | `crates/paladin-core/src/base/entity/field.rs:14-35`; exercised by `test_field_version_creation` and `test_field_version_history` (`base/service/field_version_service.rs:586`, `:607`); path and the `nid` residual are deliberately not banned (`tests/entity_rename_guard.rs:993`, fixtures at `:1020-1027`). |
| `CollectionType<T>` / `Item<T>` | A generic, uuid-identified ordered list of `Item`s (content lists, taxonomies). | Plain container with accessors (`items`, `add_item`, `remove_item`, `len`), 83 lines. Only consumer is `CollectionVersionService`. Rustdoc says unfinished and names this review. | Keep | `crates/paladin-core/src/base/entity/collection.rs:10` (`Item`), `:28` (`CollectionType`); exercised by `collection_count_changes_carry_the_nil_sentinel` and `test_collection_version_creation` (`base/service/collection_versioning_service.rs:1168`, `:634`). |
| `component::element` | A (potential) shared reference connecting entities, the "potential edge" (the `tag` example). | Comment-only module: declares no type, so there is nothing to fence. Its `# Status` section says unfinished and names this review. | Keep | `crates/paladin-core/src/base/component/element.rs:1-30` (no item declared); module declaration kept in `base/component/mod.rs`. No test pins it because it has no behaviour. |
| `FieldVersionService<T>` + `InMemoryFieldVersionRepository<T>` | Field-level revision history, opt-in per field. | Works against the in-memory repository only (no persistence). Before this phase a field with no stored version got a history with a randomly generated `node_id`. Now it is a typed refusal; a purge of such a field still returns `Ok(0)`. | Keep and fence | Refusal at `crates/paladin-core/src/base/service/field_version_service.rs:507-510` (`ok_or(VersioningError::EntityNotFound(field_id))?`). Tests `field_version_history_of_an_unversioned_field_is_entity_not_found` (`:631`) and `purging_an_unversioned_field_purges_nothing` (`:649`). Caller read: no consumer outside `base`; the only other mentions are `tests/entity_rename_guard.rs:1000-1002` fixture strings. |
| `CollectionVersionService<T>` + `InMemoryCollectionVersionRepository<T>` | Collection-level revision history with change detection. | Versioning works; change detection is count-only (no per-item diff) and in-memory only. Count changes used to carry a random item id; they now carry `Uuid::nil()`, the documented "item not tracked" sentinel. | Keep and fence | Sentinel at `crates/paladin-core/src/base/service/collection_versioning_service.rs:432` (`ItemAdded`) and `:434` (`ItemRemoved`). Tests `collection_count_changes_carry_the_nil_sentinel` (`:1168`), `comparing_the_same_collection_versions_twice_is_identical` (`:1193`), `equal_collections_report_no_change` (`:1206`). Caller read: no consumer outside `base`. |
| `planning_service.rs` expected-output placeholder | The plan schema the LLM fills carries `id`, `description` and `dependencies`; `Subtask` also has an `expected_output` field a later prompt can use. | `create_plan` used to invent an expected-output sentence for every subtask and the prompt sent it to the LLM as if it were a requirement. It now builds every subtask with an empty `expected_output`, and `execute_subtask` omits the `EXPECTED OUTPUT` line and its closing clause when empty. A caller-supplied expected output is rendered byte-for-byte as before. | Fence | `src/application/services/paladin/planning_service.rs:418-423` (`String::new()`), `:508` (empty-or-whitespace branch). Tests `create_plan_does_not_fabricate_an_expected_output` (`:729`), `subtask_prompt_omits_expected_output_when_none_is_set` (`:748`), `subtask_prompt_keeps_a_supplied_expected_output` (`:778`). Choice recorded in `55-06-SUMMARY.md` (empty string, not the description, which is already sent as `SUBTASK:`). |
| `file_content_fetcher.rs`: `ingest_content` and three metadata helpers | Take a validated `ContentItem` and store, index and thumbnail it; read video / audio duration and image dimensions from the file. | `ingest_content` used to return `Ok(())` for an ingestion that stored nothing. It now keeps its per-type URL validation, then returns `Err("FileContentFetcher::ingest_content is unfinished: content ingestion is not implemented, nothing was stored")`. `get_video_duration`, `get_audio_duration` and `get_image_dimensions` return `None` (absence, not a made-up value) and say so in rustdoc; no media library is chosen. Their three callers (`create_video_content`, `create_audio_content`, `create_image_content`) still fold that `None` into `0` / `(0, 0)`, because the content types have no "unknown" representation; the callers' comments now say so (verification follow-up, IN-11), so an unknown figure is recorded as zero, not refused. | Fence | `crates/paladin-content/src/adapters/input/file_content_fetcher.rs:16` (const), `:152-195` (`ingest_content`), `:105-118` (helpers). Test `test_ingest_content_refuses_valid_content_as_unfinished` (`:313`). Caller read: no shipped caller of `ContentIngestionPort::ingest_content` (`55-06-SUMMARY.md`; trait declaration `crates/paladin-ports/src/input/content_input_port.rs:27`). The three helpers are documented, not test-pinned: `None` was already the honest answer, so they needed rustdoc and no behaviour change. |
| The twenty `ContentListRepository` bodies | Persist named content lists (get, save, update, delete, range queries, add / remove item) on SQLite and MySQL; the G3 scaffolding of the content system. | All twenty (ten on `SqliteStore`, ten on `MySqlContentRepository`) were `todo!()` and panicked the calling thread. Each now returns `Err(RepositoryError::QueryError("ContentListRepository::<method> is unfinished for the <backend> backend: content lists are not persisted"))`. Neither backend persists content lists; the trait (`crates/paladin-ports/src/output/repository_port.rs:84-122`, ten methods) is unchanged. | Fence | `crates/paladin-storage/src/sqlite_content_repository.rs:650` (helper) and `:663` (impl); `crates/paladin-storage/src/mysql_content_repository.rs:627` and `:640`. Tests `content_list_repository_refuses_every_method_on_sqlite` (`sqlite_content_repository.rs:839`) and `content_list_repository_refuses_every_method_on_mysql` (`mysql_content_repository.rs:809`). The `scripts/check-no-stub-panics.sh` gate fails the build if a `todo!` or `unimplemented!` returns outside `#[cfg(test)]` in a published crate. No caller outside the two adapters (see below). |

### Caller re-check

`grep -rnE 'FieldVersionService|CollectionVersionService|CollectionType|base::entity::(field|collection)|base::component::element'`
over `crates`, `src`, `tests`, `examples` and `benches`, excluding `crates/paladin-core/src/base/`, matches only
`tests/entity_rename_guard.rs` (prose and fixture strings that say these paths are not banned). `grep -rn
'ContentListRepository'` over the same trees, excluding the two adapter files, matches only the trait
declaration and two `paladin-storage/src/lib.rs` doc lines. The earlier "no consumer outside `base`" and
"no caller of `ingest_content`" findings therefore hold at this head. The todo's owner position (below) means
this count is context, not the reason for any decision.

## Why nothing is removed this milestone

Every item is a `pub` item of a published crate (`paladin-ai-core`, `paladin-content`, `paladin-storage`,
`paladin-ports`). Removing one is a deliberate breaking change that needs a `MIGRATION.md` section 11.2 row (a deliberate-breaking `Y` entry), a
`.cargo/semver-checks-allowlist.toml` entry and a `cargo-semver-checks` allow, and Phase 53's version decision
(`53-VERSION-DECISION.md`) holds v0.11.1 as a patch release with no breaking item. The scope guard of this
milestone forbids a non-security break, so removal, rename and finishing are out of scope here, as is any change
of the entity-sense identifiers (`Field.nid`, the field-version `node_id` fields, `get_node_field_versions`)
that ADR-0060 (g) names as the bounded exception. The fences above change behaviour only where the old
behaviour fabricated a value, and they are recorded in `MIGRATION.md` section 11.1 (`M-B-30`, `M-B-33`,
`M-B-34`, `M-B-35`) and the `[Unreleased]` changelog entries.

## Next-minor candidates

None of these is decided or started. Each says what would trigger it.

- **Option A, finish the system.** Build the content / general-data orchestration system the base layer
  anticipated: persisted content lists on SQLite and MySQL (replacing the twenty refusals), a real
  `ingest_content`, per-item collection diffs, and platform-wide revisioning (the `Entity<T>::version` flag's
  unfulfilled promise). It is a feature milestone, so it needs a PRD first, because no design record exists.
  Trigger: the owner schedules the system as product work.
- **Options B and C, retire the system, fully or partially.** B retires every unneeded item; C keeps
  `CollectionType` / `Item` and the two versioning services and drops `Field` and `element` once
  `Battlefield`'s `FieldName` / `FieldSpec` types are confirmed as their replacement. Either is a Phase-44-style
  clean break: deliberate-breaking rows in the `MIGRATION.md` section 11.2-style X-10 register of the releasing
  version, semver allowlist entries and crate-wide allows, a
  repository-scan guard rule, and the entity-sense identifier renames the todo lists (`Field.nid`,
  `FieldVersion.node_id`, `FieldVersionHistory.node_id`, `FieldVersionComparison.node_id`,
  `get_node_field_versions`). The fenced `ContentListRepository` impls and `ingest_content` would be deleted in
  the same break. Trigger: the owner decides, in a minor-version milestone that may carry a breaking change,
  that the system will not be finished.

The todo's `recheck_by` ("before the milestone after v0.11.0 is roadmapped") is met by this record; the
decision between A, B and C is the next-minor milestone's to take, and the todo is closed on that basis.

## Owner position

Quoted verbatim from the todo's "Operator position (2026-10-13)":

> Keep everything until the review has been done; do not let a consumer count decide it.

The review has now been done for this milestone: it keeps everything and fences the fabricating bodies. It does
not use a consumer count as a reason; the caller re-check above is evidence for the fences, not for removal.
