# Phase 35 Deferred Items Register

Per D-26 and D-27, this register holds findings this phase surfaces that are neither fixed as part
of an `MB-nn` row's own closure nor absorbed silently into a neighbouring page's fix. Nothing here
is fixed in this phase — each entry stays a pointer only.

Plans 35-02 through 35-09 run in parallel worktrees and record their own observations under a
`## Deferred observations` heading in their own SUMMARY.md rather than editing this file directly;
plan 35-10 folds those SUMMARY sections into this register as its own closing task.

## Plan 35-01, Task 3

1. **`docs/src/contributing/contributing-providers.md` carries the same relocated-LLM-adapter
   import defect D-13 fixes elsewhere, at its own lines 272 and 367 — but the page is not in the
   Phase 35 work list.** `35-RESEARCH.md` Open Question 2 confirmed this live: the page's
   `paladin::infrastructure::adapters::llm::…` occurrences match exactly the defect D-13 names for
   `minio-file-repository-setup.md`, `redis-queue-adapter-setup.md`, `sanctum-migration.md`,
   `port-trait-template.md`, `provider-expansion.md` and `sentinel.md` — but
   `contributing-providers.md` does not appear anywhere in `34-AUDIT.md` §5's 60-row work list, and
   §2 row 52 settles the page `current` on every signal the audit actually checked. Per D-27, a
   defect noticed on a page the audit did not route to Phase 35 is recorded here with a proposed
   classification, not fixed silently, even though the fix would be trivial and mechanically
   identical to the five D-13 pages' own fix.
   - **Proposed classification:** a missed `MB-nn` candidate — the audit's §2 sweep settled this
     page `current` without checking for the relocated-adapter import path specifically (the same
     signal class the D-13 five pages failed on). A future documentation pass (or a Phase 35
     follow-up quick task) should re-run the `34-signals.sh contributing/contributing-providers.md`
     nine-signal check against this one page and, if it reproduces the two-line defect, apply the
     identical D-13 fix (`paladin_ports::output::…` / `paladin_llm::{openai,anthropic,deepseek}::…`
     with the `OpenAIAdapter` casing) in its own commit.
   - **Owner:** unassigned — not Phase 35 (out of its minted work list) and not Phase 36 (Phase 36's
     scope is rustdoc warnings, intra-doc links, `examples/` programs and existing `doc-examples`
     module edits per `35-CONTEXT.md`'s Phase Boundary — not new `docs/src` prose fixes). A future
     docs-currency pass or a standalone quick task is the natural owner.

2. **Standing rule for Phase 36 `EX-nn` pointers (D-26).** Any page fix in plans 35-02 through
   35-09 that would need an *existing* `crates/doc-examples` module's anchors or `support.rs`
   changed (rather than a wholly new module Phase 35 is permitted to add) is Phase 36's territory,
   never Phase 35's — Phase 35 only adds modules and `lib.rs` registrations (D-26). Record such an
   observation here as `page | existing anchor/module | what the page wanted changed | why it was
   left as an include with no edit`, so Phase 36 can pick it up as an `EX-nn` row without having to
   re-discover it from scratch. No such case was found by plan 35-01 itself (`superstep_engine.rs`
   is a wholly new module); this entry documents the rule for the plans that follow.

## Plan 35-10 fold-in — deferred observations from plans 35-02 through 35-09

Plan 35-10 (phase close) collected every `## Deferred observations` SUMMARY section across the
eight parallel wave-2/wave-3 plans and folds them in below. Plans 35-02, 35-03, 35-05 and 35-08
recorded **no** deferred observations — each stated explicitly in its own SUMMARY that it touched
no page the audit had settled `current`, so there is nothing to fold from those four plans.

### From plan 35-04 — `docs/src/contributing/adr-index.md:14` (RESOLVED before this fold)

**Observation as recorded:** plan 35-04's Task 2 `<verify>` re-ran the SC4 vocabulary exit grep
phase-wide (`grep -rqiE '\bQuartermaster\b' docs/src`) and found a second, independent occurrence
on `docs/src/contributing/adr-index.md:14` (the ADR-0049 summary row's "never as `Quartermaster`"
phrasing) — a page plan 35-05 created after plan 35-04's own scope assumption ("Commissary.md is
the single remaining source") was written. Recorded in `.planning/WINDOWS.md` (`kind:
unmet-truth`) for this plan to fold in and fix.

- **Status:** **closed**, not open. The orchestrator's cross-plan integration commit `64a44c51`
  ("fix(35): drop the retired name from the ADR-0049 index row (D-17, cross-plan integration fix
  after wave 2)") reworded the line after wave 2 merged, before this plan's own work began. Live
  re-check this plan performed (`grep -rniE '\bQuartermaster\b' docs/src` → empty; `sed -n '14p'
  docs/src/contributing/adr-index.md` → "Re-ported under the new vocabulary; the retired name and
  the rejected alternatives are recorded in the ADR itself") confirms the fix landed and the SC4
  exit grep is fully empty book-wide as of this plan's `35-EVIDENCE.md` capture.
- **Proposed classification:** n/a — resolved, no further action.
- **Owner:** n/a — closed by `64a44c51`.

### From plan 35-06 — `docs/src/contributing/testing-guide.md:96`'s stale fixture-path claim

**Observation as recorded:** the `tests/` directory-structure ASCII tree on `testing-guide.md`
lists `fixtures/config.test.yml`, but the real fixture (`config.test.yml`) lives at the repository
root, not under `tests/fixtures/`. This is a real, pre-existing minor inaccuracy in the tree,
unrelated to any fabricated CI workflow name (it is why the tree trips the phase-level D-21
fabricated-CI-name grep as a false positive — see the allowlist row in `35-EVIDENCE.md`). Out of
scope for MB-36's task instructions, which named only the CI/coverage sections.

- **Proposed classification:** a small, self-contained content fix — one line in an ASCII tree.
  Not an `MB-nn` row (MB-36 already closed `testing-guide.md`'s CI/coverage scope); a future
  documentation quick-task can move the `config.test.yml` line out of the `fixtures/` branch of
  the tree (or add a one-line note that it lives at the repo root).
- **Owner:** unassigned — not Phase 35 (outside MB-36's stated scope) and not Phase 36 (Phase 36's
  scope is rustdoc/intra-doc links, `examples/` programs and existing `doc-examples` module edits,
  not new `docs/src` prose fixes). A future docs-currency pass or a standalone quick task is the
  natural owner.

### From plan 35-07 — `docs/src/appendix/cli-configuration.md`'s Garrison and Arsenal troubleshooting entries

**Observation as recorded:** while closing MB-40 (the Scheduler troubleshooting entry's stale "no
TODO at line 297" claim), the Garrison and Arsenal troubleshooting entries nearby make the
identical claim style ("verify no TODO at line NNN"). A live check
(`grep -rn "TODO.*garrison\|garrison.*TODO\|TODO.*arsenal\|arsenal.*TODO"
src/application/cli/commands/agent.rs`) also returns zero hits, suggesting these two entries may
be equally stale — but no `MB-nn` row covers them, and the audit's MB-40 finding named only the
Scheduler entry.

- **Proposed classification:** a missed `MB-nn` candidate, same shape as the 35-01 observation
  above (a signal the audit's §2 sweep did not check for on this specific page, for these specific
  entries). A future pass should confirm the TODO claims are indeed stale on both entries and
  rewrite them the same way MB-40 rewrote the Scheduler entry (pointing at the live route family
  the CLI command actually calls, rather than a source-line TODO check).
- **Owner:** unassigned — same reasoning as above: out of Phase 35's minted work list, out of
  Phase 36's stated scope. A future docs-currency pass or a standalone quick task is the natural
  owner.

### From plan 35-09 — `docs/src/appendix/battalion-patterns-guide.md`'s body content beyond the four opening imports

**Observation as recorded:** MB-38's cited finding and plan 35-09's own task scope were explicit —
correct only the four opening `use paladin::battalion::*;` import lines to the compiling facade
path. The body of each of the four examples on the same page still carries the `OpenAiAdapter`
casing bug (not `OpenAIAdapter`) and other pre-existing API-shape drift, left untouched per that
explicit scope. The same `OpenAiAdapter` casing bug also appears on five pages entirely outside
this phase's `files_modified` lists: `docs/src/contributing/testing-guide.md`,
`docs/src/appendix/battalion-vision-support.md`, `docs/src/appendix/conclave-pattern.md`,
`docs/src/contributing/architecture-decisions.md`, `docs/src/user-guides/memory-management.md`,
`docs/src/user-guides/tool-integration.md`.

- **Proposed classification:** a casing-consistency defect (systemic, six-plus pages), not itself
  an `MB-nn` row — none of `34-AUDIT.md` §5's sixty rows names this specific defect class. A
  future documentation pass should grep `OpenAiAdapter\b` (word-boundary, to avoid matching the
  correct `OpenAIAdapter`) book-wide and correct every occurrence to the live struct's casing in
  one sweep, the same way D-13 closed the `paladin::paladin_ports::` double-nesting defect in this
  phase.
- **Owner:** unassigned — outside Phase 35's minted MB-nn work list (no row names this defect) and
  outside Phase 36's stated scope (rustdoc/intra-doc links, `examples/` programs, existing
  `doc-examples` module edits — not new `docs/src` prose casing fixes). A future docs-currency pass
  or a standalone quick task is the natural owner.

---

*Phase: 35-mdbook-currency*
*Register opened: 2026-09-17, plan 35-01*
*Folded and closed: 2026-09-17, plan 35-10 (phase close)*
