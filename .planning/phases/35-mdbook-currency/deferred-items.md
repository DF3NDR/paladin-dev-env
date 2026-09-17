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

---

*Phase: 35-mdbook-currency*
*Register opened: 2026-09-17, plan 35-01*
