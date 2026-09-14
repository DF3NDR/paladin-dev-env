---
phase: 30-token-economy-vocabulary-commissary-anchoring
plan: 02
subsystem: docs
tags: [adr, planning-record, treasurer, versioning, x-03-supersession]

# Dependency graph
requires:
  - phase: 30-token-economy-vocabulary-commissary-anchoring
    provides: "ADR-0049 (Commissary design), the ADR numbering/heading skeleton at PROMOTION.md, PROJECT.md's Key Decisions table with the ADR-0049 row already appended"
provides:
  - "ADR-0050: Treasurer reserved for cross-run spend governance, with the downstream GarrisonTreasury guardrail"
  - "ADR-0051: Phases 31-33 land as clean breaks inside the untagged v0.10.0, superseding corpus rule X-03 for those three phases only"
  - "PROMOTION.md indexed through ADR-0051, Next free ADR number at 0052"
  - "PROJECT.md Key Decisions links all three of this phase's ADRs in ascending order"
affects: [31-token-carrier-change, 32-commissary-signature-drop, 33-commissary-in-tree-adoption, Milestone_14-Treasurer]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reservation-shape ADR (0050): reserves a term with 0/0 in-tree verification and states a durable symbol-scoped invariant plus the exact prose-only successor state a later plan in the same phase will create, so the record does not age into a false claim."
    - "Supersession-in-Decision ADR (0051): narrows a program-corpus rule (X-03, not itself an ADR) for a named, bounded set of future phases without a ## Supersedes heading, since that heading only applies when superseding another ADR."

key-files:
  created:
    - .planning/decisions/0050-treasurer-reservation.md
    - .planning/decisions/0051-token-economy-versioning-x03-supersession.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md

key-decisions:
  - "ADR-0050 states the reservation's durable invariant as symbol-scoped (0/0 as a struct/enum/trait/mod/fn/impl/use/type declaration or path-qualified use), not the weaker bare-word claim — because plan 30-03 (later in this same phase) adds the term's first rustdoc-prose occurrence in herald.rs, and the ADR names that successor state up front rather than letting a naive re-grep read as the ADR having lied."
  - "ADR-0051 states the supersession inside ## Decision rather than adding a ## Supersedes heading, because X-03 is a program-corpus rule (.project/v0.10.0/00-program-overview.md), not itself an ADR — PROMOTION.md's supersession mechanism only applies when one ADR replaces another ADR."
  - "PROMOTION.md's dated note for this plan explicitly accounts for the two-step advance (0050 -> 0052) as one plan authoring two ADRs, distinguished from plan 30-01's separate one-step advance note (0049 -> 0050), so a reader sees exactly which plan is responsible for each unit of the counter's movement."

requirements-completed: [VOCAB-04, VOCAB-07]

coverage:
  - id: D1
    description: "ADR-0050 exists as a one-page reservation with the seven required headings, records the Treasurer's future scope (allowances, pricing, cost_estimate production, pacing), the installs-not-replaces rule with its install point (limits.rs), the Milestone 14 owner, the authoring-time 0/0 grep plus the durable symbol-scoped invariant and herald.rs successor-state note, and the downstream GarrisonTreasury guardrail with both rejected alternatives (Paymaster, Comptroller)."
    requirement: "VOCAB-04"
    verification:
      - kind: other
        ref: "Task 1's own <verify> automated command (heading sequence, Date/conforms/installs/limits.rs/Milestone 14/no-Epic-5/GarrisonTreasury/Paymaster/Comptroller/v0.10.0/no-v0.11.0/herald.rs, symbol-scoped grep 0/0, bare-word non-doc-line check) — re-run at Self-Check time"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-0051 exists with the seven required headings (no ## Supersedes), cites X-03 at its source line, scopes the supersession to Phases 31, 32 and 33 exactly with the exception stated as extending to no other phase, preserves the MIGRATION.md 9.2 row and semver-allowlist-row requirement as documentation rather than a shim, and states that Phase 30 itself registers neither."
    requirement: "VOCAB-07"
    verification:
      - kind: other
        ref: "Task 2's own <verify> automated command (heading sequence, Date/conforms/X-03/00-program-overview.md/31/32/33/v0.10.0, zero .rs files in the commit's diff) — re-run at Self-Check time"
        status: pass
    human_judgment: false
  - id: D3
    description: "PROMOTION.md indexes both ADRs (rows 0050, 0051), reads Next free ADR number: 0052 on exactly one line with one dated 2026-09-14 note for this plan; PROJECT.md's Key Decisions table links all three of this phase's ADRs (0049, 0050, 0051) in ascending order with no pre-existing row disturbed."
    requirement: "VOCAB-04, VOCAB-07"
    verification:
      - kind: other
        ref: "Task 3's own <verify> automated command (Next-free-ADR-number count and value, row counts, dated note, PROJECT.md ADR-link counts and ascending line-number ordering, ls of the three ADR files, zero .rs files in the commit's diff)"
        status: pass
    human_judgment: false

# Metrics
duration: 35min
completed: 2026-09-14
status: complete
---

# Phase 30 Plan 02: Treasurer Reservation & X-03 Versioning Supersession Summary

**ADR-0050 reserves the `Treasurer` spend-governance officer term with the downstream
`GarrisonTreasury` guardrail; ADR-0051 records that Phases 31-33 supersede corpus rule X-03 as
clean breaks inside the untagged v0.10.0; both are indexed in `PROMOTION.md` and linked from
`PROJECT.md`'s Key Decisions table.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-09-14T19:05:00Z (approx.)
- **Completed:** 2026-09-14T19:40:00Z (approx.)
- **Tasks:** 3
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments
- `.planning/decisions/0050-treasurer-reservation.md` (new) reserves the output-side,
  cross-run spend-governance officer name: what it will own (allowances, per-model pricing,
  `cost_estimate` production, pacing), what it does NOT do (installs, not replaces, the
  per-run `TokenBudget` at `src/application/services/paladin/middleware/limits.rs`), when it
  is built (Milestone 14, hard-depends on Phase 31), and the downstream guardrail (the term is
  framework-only, never used as an audit-target or fixture domain term in the downstream Web3
  Security Paladin app, which already has a colliding `GarrisonTreasury` fixture). The
  authoring-time `grep -rn Treasurer crates src` (2026-09-14, commit `2c581d95`) returned no
  matches; the ADR states the durable invariant as symbol-scoped (0/0 as a code symbol,
  permanently) and names `herald.rs` as the one place plan 30-03 will later add a rustdoc-prose
  occurrence, so a re-grep after Phase 30 closes does not read as the ADR having lied.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` (new) records that
  v0.10.0 corpus rule X-03 (`.project/v0.10.0/00-program-overview.md:44`) is superseded for
  Phases 31, 32 and 33 only, on the operator's 2026-09-14 decision — naming the two breaks the
  milestone already anticipates (the token-carrier change, Phase 31; dropping
  `Commissary::new`'s `is_exact_counter` argument, Phase 32) — while every break still gets a
  `MIGRATION.md` §9.2 row and a `cargo semver-checks` allowlist row as documentation for the
  downstream refactor, never as a compatibility shim. States explicitly that Phase 30 itself
  registers zero of either.
- `PROMOTION.md` now indexes all three of this phase's ADRs (0049, 0050, 0051), reads
  `**Next free ADR number: 0052**` on exactly one line, and carries a dated 2026-09-14 note for
  this plan distinguishing its two-step advance from plan 30-01's separate one-step advance.
- `.planning/PROJECT.md`'s Key Decisions table gains two new rows (ADR-0050, ADR-0051) appended
  below the ADR-0049 row in ascending order, each linking rather than restating the ADR.

## Task Commits

Each task was committed atomically:

1. **Task 1: ADR-0050 — reserve the spend-governance officer term, with the downstream guardrail** - `ac765bb7` (docs)
2. **Task 2: ADR-0051 — clean breaks inside the untagged v0.10.0, superseding X-03 for Phases 31-33** - `7e75e388` (docs)
3. **Task 3: Index bookkeeping — PROMOTION.md rows and the PROJECT.md Key Decisions rows** - `b0d1dc5e` (docs)

## Files Created/Modified
- `.planning/decisions/0050-treasurer-reservation.md` - New ADR: Treasurer reservation, installs-not-replaces rule, Milestone 14 owner, downstream guardrail
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` - New ADR: X-03 superseded for Phases 31-33 only, migration-register documentation preserved
- `.planning/decisions/PROMOTION.md` - Index rows for ADR-0050/0051; `Next free ADR number` advanced to 0052; dated note
- `.planning/PROJECT.md` - Two new Key Decisions rows linking ADR-0050 and ADR-0051, ascending order

## Decisions Made
- ADR-0050's durable invariant is stated as symbol-scoped (no `struct`/`enum`/`trait`/`mod`/`fn`/
  `impl`/`use`/`type` declaration and no path-qualified use), not the weaker bare-word 0/0 claim,
  because plan 30-03 (later in this phase) is known in advance to add a rustdoc-prose occurrence
  in `herald.rs` — the ADR anticipates that successor state rather than being falsified by it.
- ADR-0051 states its supersession inside `## Decision` prose rather than adding a
  `## Supersedes` heading, because PROMOTION.md's supersession mechanism applies only when one
  ADR replaces another ADR — X-03 is a program-corpus rule, not an ADR, so the seven-heading set
  stays intact with no eighth heading.
- Each ADR's `## Considered Options` and `## Code Locations` bullets are single physical lines
  (no wrapped continuation), so every content line under those headings begins with `- ` — this
  matters both for `adr-parser.cjs`'s `splitEntries` contract and for the plan's own line-format
  acceptance criterion, which checks every line, not just the first line of each bullet.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- VOCAB-04 and VOCAB-07 are both satisfied: the Treasurer term is reserved in writing with the
  downstream guardrail, and Phases 31-33's clean-break assumption now rests on a citable ADR
  rather than an operator conversation and a milestone-overview paragraph.
- `PROMOTION.md` reads `Next free ADR number: 0052`; the next phase or plan authoring an ADR
  takes 0052 without needing to re-derive it.
- No blockers. All three commits touch no `.rs` file, no `Cargo.toml`/`Cargo.lock`, and no
  `MIGRATION.md` path; the reserved `Treasurer` term remains 0/0 as a code symbol across
  `crates/` and `src/`.
- Ready for plan 30-03 (the `max_tokens` disambiguation table, the `cost_estimate` rustdoc
  reservation naming this plan's Treasurer role, and the Quartermaster purge).

## Self-Check: PASSED

Files verified to exist on disk:
- FOUND: `.planning/decisions/0050-treasurer-reservation.md`
- FOUND: `.planning/decisions/0051-token-economy-versioning-x03-supersession.md`
- FOUND (modified): `.planning/decisions/PROMOTION.md`
- FOUND (modified): `.planning/PROJECT.md`

Commits verified in `git log --oneline`:
- FOUND: `ac765bb7` — `docs(30-02): reserve the Treasurer term with ADR-0050`
- FOUND: `7e75e388` — `docs(30-02): record the X-03 clean-break supersession as ADR-0051`
- FOUND: `b0d1dc5e` — `docs(30-02): index ADR-0050 and ADR-0051 and link them from Key Decisions`

Verification commands re-run at Self-Check time:
- Task 1's full automated `<verify>` command (heading sequence, Date/conforms/installs/
  limits.rs/Milestone 14/no-Epic-5/GarrisonTreasury/Paymaster/Comptroller/v0.10.0/no-v0.11.0/
  herald.rs, symbol-scoped grep, bare-word non-doc-line check) — exit 0 (VERIFY_PASS)
- Task 2's full automated `<verify>` command (heading sequence, Date/conforms/X-03/
  00-program-overview.md/31/32/33/v0.10.0, zero `.rs` files in commit diff) — all conditions pass
- Task 3's full automated `<verify>` command (Next-free-ADR-number count/value, row counts,
  dated note, PROJECT.md ADR-link counts and ascending ordering, `ls` of three ADR files, zero
  `.rs` files in commit diff) — all conditions pass
- Plan-level cross-commit check: `git diff --name-only 2c581d95..HEAD` lists exactly
  `.planning/PROJECT.md`, `.planning/decisions/0050-treasurer-reservation.md`,
  `.planning/decisions/0051-token-economy-versioning-x03-supersession.md`,
  `.planning/decisions/PROMOTION.md` — zero `.rs`/`Cargo.toml`/`Cargo.lock`/`MIGRATION.md` paths
- `grep -rnE '\b(struct|enum|trait|mod|fn|impl|use|type) +Treasurer\b|\bTreasurer::' crates src`
  — exit 1 (no match); `grep -rn 'Treasurer' crates src | grep -v '///' | grep -v '//!'` — empty

---
*Phase: 30-token-economy-vocabulary-commissary-anchoring*
*Completed: 2026-09-14*
