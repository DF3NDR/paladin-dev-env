# Phase 48: Base Entity Rename - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-13
**Phase:** 48-base-entity-rename
**Areas discussed:** Old name's fate, Names and ergonomics, Base-layer disposition, Record and delivery

**Starting point:** a draft 48-CONTEXT.md (code survey, recommendations D-01..D-07, four open
questions) and nine plans written the same morning that had accepted every recommendation
as-is and added planner resolutions D-08..D-14. Nothing had executed. The owner chose "Update
it" and "Continue and replan after". Three keyword-matched todos were reviewed and none folded.

---

## Old name's fate

| Option | Description | Selected |
|--------|-------------|----------|
| Clean break, no alias | Remove `Node<T>` and its module; ADR-0060 supersedes X-03 for Phase 48; §9.7 stays empty | ✓ |
| Deprecated alias for one release | X-03 as written; alias cannot cover the renamed field, so downstream breaks anyway | |

**User's choice:** Clean break, no alias.

| Option | Description | Selected |
|--------|-------------|----------|
| Key becomes `data`, reads accept `node` | `#[serde(alias = "node")]`; v0.11 writes only the new key; downgrade asymmetry documented | ✓ |
| Key stays `node` on the wire | `#[serde(rename = "node")]`; persisted documents byte-identical, keep the old word forever | |

**User's choice:** Key becomes `data`, reads accept `node`.

| Option | Description | Selected |
|--------|-------------|----------|
| Whole 0.x line, own ADR to remove | One attribute per field; low risk of forgetting | ✓ |
| Remove at v0.12.0, scheduled in §9.7 | Makes §9.7 non-empty; forces a v0.12 cleanup phase | |
| You decide | | |

**User's choice:** Whole 0.x line, own ADR to remove.
**Notes:** "Next area" after three questions.

---

## Names and ergonomics

| Option | Description | Selected |
|--------|-------------|----------|
| `Entity<T>` / `.data` | Generic on purpose; base layer predates the theme | ✓ |
| `Entity<T>` with a different field name | payload / inner / value | |
| A different wrapper name | | |

**User's choice:** `Entity<T>` / `.data`.

| Option | Description | Selected |
|--------|-------------|----------|
| `entity` | `item.entity.data.title`; removes the last entity-sense "node" from public fields | ✓ |
| Keep `node` on these two structs | Smallest diff; two public fields keep the old word; guard carve-out | |
| `inner` | Neutral idiom, less self-describing | |

**User's choice:** `entity`.

| Option | Description | Selected |
|--------|-------------|----------|
| `entity_id` / `entity_data` / `EntityNotFound` | With serde aliases; confined to four files | ✓ |
| Keep `node_id` / `node_data` / `NodeNotFound` | Leaves an entity-sense `node_id` beside 1,151 graph-sense ones | |
| You decide | | |

**User's choice:** `entity_id` / `entity_data` / `EntityNotFound`.
**Notes:** "Next area".

---

## Base-layer disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Remove all five | ~1,800 dormant lines; 0 consumers; coverage effect < 0.4 pp | |
| Keep some for the downstream system | Each kept item with a written reason | |
| *(free text)* | Keep **all five**: they belong to the unfinished content / general-data-orchestration system; a follow-up todo must review that system before anything is removed | ✓ |

**User's choice:** Free text — keep all five, open a review todo.
**Notes:** Todo created at
`.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`. This
overrides the survey's D-05 recommendation and voids old plan 48-03's removal tasks.

| Option | Description | Selected |
|--------|-------------|----------|
| Keep the name, add a rustdoc contract | No wire change, no extra break | ✓ |
| Remove the flag | Another public break; ContentItemService needs a new signal | |
| Rename it | Field break and alias for no behaviour change | |

**User's choice:** Keep the name, add a rustdoc contract.

| Option | Description | Selected |
|--------|-------------|----------|
| Delete the alias only | Keep the placeholder `PaladinData` / `PaladinStatus`; no wire change | |
| Unify with `paladin.rs`'s `PaladinData` | Removes the placeholder types; second public break on `PaladinState.paladin`'s type | ✓ |

**User's choice:** Unify with `paladin.rs`'s `PaladinData`.
**Notes:** Overrides the survey's D-10 and 48-RESEARCH.md Pitfall 4's "do not unify". Verified
after the answer: both structs have the same 13 fields in the same order; the real one adds
`#[serde(default)]` on four and its `PaladinStatus` adds `Reasoning`; five external files import
the placeholder types. "Next area".

---

## Record and delivery

| Option | Description | Selected |
|--------|-------------|----------|
| Milestone branch, same PR as 44-47 | One PR to `main` carries the whole v0.11.0 milestone | ✓ |
| Own PR to `main` now via `gsd-pr-branch` | Isolated review, but the branch carries Phase 44 beneath it | |

**User's choice:** Milestone branch, same PR as 44-47.

| Option | Description | Selected |
|--------|-------------|----------|
| One ADR-0060 covering all three | Supersession + vocabulary rule + serde carve-out, ADR-0059 shape | ✓ |
| Two ADRs: 0060 supersession + 0061 vocabulary | Vocabulary rule findable on its own; second PROMOTION row | |

**User's choice:** One ADR-0060 covering all three.

| Option | Description | Selected |
|--------|-------------|----------|
| Ban old paths AND pin graph definition sites | File-scoped "still declared here" rules for every graph identifier | ✓ |
| Ban old entity paths only | Graph freeze asserted by ADR and docs only | |

**User's choice:** Ban old paths AND pin graph definition sites.

| Option | Description | Selected |
|--------|-------------|----------|
| Re-run `/gsd-plan-phase 48` from the updated CONTEXT | Regenerate all plans with the checker loop | ✓ |
| Targeted edits to 48-03, 48-04, 48-07, 48-09 | Faster, easy to leave must_haves and waves inconsistent | |

**User's choice:** Re-run `/gsd-plan-phase 48` from the updated CONTEXT.
**Notes:** Final check "Which gray areas remain unclear?" → "I'm ready for context".

---

## Claude's Discretion

- `Entity<T>`'s private module name (`wrapper.rs`) and the `entity_version_service` module name.
- Rename stage order and commit granularity; fixture set beyond the four named; when the D-27
  semver diagnostic runs.
- Docs-only wording fixes for inherited errors in ENTITY-01/04/05 at closeout, IDs unchanged.
- Wording of the retained-system rustdoc in `base/mod.rs`.

## Deferred Ideas

- Removing any retained base item (gated on the new review todo).
- Platform-wide revisioning; splitting `platform/container/`; downstream migration; the `Log`
  alias name; removing the serde read-aliases.
- Reviewed, not folded: `2026-10-11-phase-44-review-follow-ups.md`,
  `2026-10-11-reject-removed-v0-10-battalion-config-keys.md`,
  `2026-08-13-verify-local-coverage-reproduction.md`.
