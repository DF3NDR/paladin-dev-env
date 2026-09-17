# Phase 35: mdBook Currency - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-17
**Phase:** 35-mdbook-currency
**Mode:** `--auto` — every question below was resolved by selecting the recommended option without
a user prompt. Each row marked ✓ is the auto-selected default.
**Areas discussed:** Page disposition tiers, The superstep-engine page (MB-30), Snippet
verification and correction style, Vocabulary items, Version and MSRV sweep, Closure evidence /
commits / CHANGELOG / Phase 36 boundary

---

## Page disposition tiers (25 appendix rows, MB-35, MB-16)

| Option | Description | Selected |
|--------|-------------|----------|
| Three tiers: correct / archive with ADR-0047 banner / retitle; never delete | Live-reference pages are corrected to the tree; dated snapshots and completion summaries get the archive banner plus their cited one-line fixes; nav/content mismatches are retitled. No page or nav entry removed. | ✓ |
| Rewrite every appendix page to the current tree | Closes every ID by content, but rewrites 25 pages the M11 non-goal exempted and regenerates reports Phase 36 will immediately invalidate. | |
| Delete the pre-milestone artifacts and drop them from SUMMARY.md | Simplest for linkcheck, but contradicts ADR-0047's "archiving records a disposition, it does not destroy the record" and breaks any inbound link. | |

**Auto-selected:** three tiers.
**Notes:** `doc-coverage-report.md` archive-vs-regenerate (left to this phase by the Phase 34
deferred note) → archive, banner pointing at ADR-0033 and Phase 36. `user-rest-api.md`'s malformed
structure is repaired only enough to render. PROJECT.md's Milestone 11 non-goal is not reopened;
ROADMAP SC1 governs, and tiering keeps effort proportional.

### MB-35 — architecture-decisions.md is an adapter guide under an "Architecture Decisions" title

| Option | Description | Selected |
|--------|-------------|----------|
| Retitle the nav entry to match the content and add a small ADR index page in the vacated slot | Keeps the accurate adapter content at its path (no link breaks) and makes the nav promise true with a one-table page linking the user-relevant ADRs by GitHub blob URL. | ✓ |
| Retitle only | S-sized, but leaves the repository's real ADRs unindexed in the book; the audit sized the row L for this reason. | |
| Rewrite the page into an ADR narrative and move the adapter guide elsewhere | Largest edit; relocates content that is currently correct. | |

### MB-16 — migration-guide.md self-contradiction

| Option | Description | Selected |
|--------|-------------|----------|
| Correct the opening line and Timeline "Current" row to v0.10.0; keep the pointer-only design | Matches the audit's D-09 finding that the page deliberately does not duplicate §9.1/§9.8. | ✓ |
| Fold the page into upgrading.md | Removes a nav entry and a link target; out of proportion for an S row. | |

---

## The superstep-engine page (MB-30)

| Option | Description | Selected |
|--------|-------------|----------|
| `user-guides/superstep-engine.md`, title "WarEngine: Battlefield State & Superstep Execution", nav after Maneuver Flow DSL and before Control Flow | Sibling naming (no article), the audit's nav position, a `Since: v0.10.0` header. | ✓ |
| Keep the audit's placeholder path `the-superstep-engine.md` | The audit said the filename was a placeholder; the article breaks the sibling convention. | |
| Place the page under Architecture instead of User Guides | It is a how-it-runs guide with runnable snippets, like Control Flow and Aegis; the dependents that link to it are user guides. | |

### Snippet strategy for the new page

| Option | Description | Selected |
|--------|-------------|----------|
| New `crates/doc-examples/src/superstep_engine.rs` module with anchors, included via `{{#include}}` | Compile-verified (SC3), the house mechanism, one narrative program (build graph → limits → run → waypoints). | ✓ |
| Prose plus `rust,ignore` fragments only | Faster, but the one L page in the phase would be the one with no compile-verified code. | |

### Scope

Locked to the audit's list (WarGraph/Battlefield/superstep merge, Waypoint checkpointing and
addressing, three WaypointPort backends, EngineConfig/EngineLimits and `APP_ENGINE_*`,
WaypointRetentionService, fingerprint versioning). Routing, Parley, Aegis and middleware are linked,
not re-explained. Four dependents (control-flow, introduction, overview, domain-model) link to it
under their own IDs.

---

## Snippet verification and correction style

### Anchor vs `rust,ignore`

| Option | Description | Selected |
|--------|-------------|----------|
| Bright-line rule: runnable flows / API calls on guide pages → doc-examples anchor; fragments → corrected + `rust,ignore`; appendix samples → corrected, scratch-compiled by the executor, `rust,ignore` | Meets SC3 literally, bounds doc-examples growth, and reuses the audit's own scratch-compile proof for appendix code. | ✓ |
| Move every corrected sample into doc-examples | Strongest guarantee, but ~20 new modules for reference pages nobody runs; Phase 36 shares the crate. | |
| Fix in place and mark everything `rust,ignore` | Cheapest; leaves the guide pages' signature-level fixes unverified, which is exactly how they drifted. | |

### The `paladin::paladin_ports::` double-nesting defect (5 pages) and relocated adapter paths (2 pages)

| Option | Description | Selected |
|--------|-------------|----------|
| One import rule established once by the researcher, applied identically; exit greps | The audit reproduced the same error on every page; one rule is re-derivable and grep-checkable. | ✓ |
| Fix page by page as encountered | Risks five different spellings of the same fix. | |

### CLI family flags (7 pages)

| Option | Description | Selected |
|--------|-------------|----------|
| Replace syntax/options blocks with captured `--help` output in captioned ```text fences; state the `--features cli` build requirement once per page | Mechanical, re-derivable, removes every fabricated flag and env var at once. | ✓ |
| Hand-correct each flag table against `paladin-cli.rs` | Same truth source, but hand transcription is how the fabricated flags appeared. | |

### Fabricated CI/release YAML (cicd.md, testing-guide.md)

| Option | Description | Selected |
|--------|-------------|----------|
| Table of the real jobs (required vs advisory) plus verbatim captioned excerpts only; codeql.yml described as security.instructions.md words it | No invented job can survive; YAML blocks still pass check-doc-config.sh. | ✓ |
| `{{#include}}` the whole workflow files | ~26 jobs of YAML on a guide page; unreadable. | |
| Delete the samples | Loses the explanation readers come for. | |

### Dated "Corrected …" callouts

| Option | Description | Selected |
|--------|-------------|----------|
| Clean corrections; retire superseded 2026-08-24 callouts; keep those still true | The audit found callouts whose premise became false — dated callouts are audit trail, and the audit trail now lives in 34-AUDIT.md, the commits and the CHANGELOG. | ✓ |
| Add "Corrected 2026-09-…" callouts per fix (Phase 16 style) | House precedent, but accumulates and rots. | |

---

## Vocabulary items

### MB-02 — the `Quartermaster` ADR-pointer sentence

| Option | Description | Selected |
|--------|-------------|----------|
| Reword so the literal token is gone; no allowlisted exception | SC4 and Phase 30 D-14 are literal; ADR-0049 still records the old name. | ✓ |
| Keep the sentence and declare it an allowlisted historical exception | Honest, but makes the exit grep non-empty forever and invites a second exception. | |

### MB-04 — introduction.md's partial fourth term list

| Option | Description | Selected |
|--------|-------------|----------|
| Cut to a labelled excerpt (≤ 8 terms, spelled as domain-model.md spells them) linking the full table | Keeps the three lists three; the intro stays an intro. | ✓ |
| Expand to a full list mirroring domain-model.md | A fourth complete list that will drift again. | |
| Remove the table | Loses the one place a newcomer meets the theme on page one. | |

### MB-03 / MB-11 — domain-model.md

Live `GarrisonEntry` struct in a `rust,ignore` fence with the source path; no `TokenUsage` rewrite
(the audit's disposition: a Garrison field, not a bare total). Add Battlefield, Waypoint, Aegis,
TraceRecord entries linking to their guides.

---

## Version and MSRV sweep

| Option | Description | Selected |
|--------|-------------|----------|
| Literal `"0.10.0"` pins, MSRV 1.88, real Dockerfile base image; named allowlist for historical tables; exit greps in the last plan | Matches what doc-examples' Cargo.toml uses and what SC3 measures against; the allowlist makes "historical" explicit rather than silent. | ✓ |
| Caret shorthand `"0.10"` | Less churn next release, but diverges from the compiled crate's own manifest and the audit's measured target. | |
| Replace pins with "the current workspace version" prose | Not copy-pasteable; Getting Started readers need a literal. | |

Feature-flag and crate tables regenerated from `Cargo.toml [features]` rather than patched, so
every shipped flag appears once; both crate-map mermaid graphs gain `mem --> llm`.

---

## Closure evidence, commits, CHANGELOG, Phase 36 boundary

### Proving each ID closed

| Option | Description | Selected |
|--------|-------------|----------|
| Per-plan closure table + one commit per page tagged `(MB-nn)` + `35-EVIDENCE.md` with the docs.yml run and exit greps | `git log --grep MB-` reproduces the closure map; VERIFICATION cites IDs against commits (Phase 34 D-03). | ✓ |
| One squash commit per wave, closure table only in the final SUMMARY | Fewer commits, but the ID→commit map becomes prose. | |

### CHANGELOG entry (SC5)

| Option | Description | Selected |
|--------|-------------|----------|
| `### Documentation` subsection after `### Fixed`, reader-facing bullets per nav section, no MB IDs; Phase 36 appends | Follows the 0.5.0 precedent and Keep-a-Changelog's reader focus. | ✓ |
| Bullets under `### Fixed` | Mixes doc corrections into code fixes. | |
| One bullet per MB ID | Sixty planning identifiers in a release note. | |

### Shared `crates/doc-examples` with Phase 36

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 35 only adds modules; existing-anchor changes are recorded as Phase 36 pointers in a phase-local deferred-items.md | Zero overlap in files both phases edit; `lib.rs` additions merge trivially. | ✓ |
| Allow Phase 35 to edit existing anchors when a page needs it | Guaranteed merge conflicts with EX-nn fixes on the same lines. | |

---

## Claude's Discretion

- Plan count and waves (suggested four: MB-30 first; guides + reference in parallel; appendix;
  CHANGELOG + evidence).
- Banner wording per archive-tier page; the exact ADR set in the index page; tier assignment for
  appendix pages D-02/D-03 do not name; whether callout removals ride in their page's commit; the
  ADR index page's nav position within Contributing.

## Deferred Ideas

- Regenerating `doc-coverage-report.md` from a real measurement (after Phase 36).
- Existing doc-examples anchor changes surfaced by page fixes → Phase 36 pointers.
- An ADR chapter inside `docs/src/`; a feature-flag table generator; PROJECT.md corrections;
  `make doc` failing on rustdoc warnings (Phase 36 SC2).

## Todos reviewed, not folded

- Coverage-reproduction walk (2026-08-13) — docs slice already closed by Phase 34 (MB-36);
  remainder is infrastructure verification.
- MinIO → RustFS evaluation (2026-09-13) — keyword false positive; zero MB items from the
  object-store sweep; infrastructure decision. Same call as Phases 32-34.
