# Phase 46: Docs Currency & Hygiene - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-24
**Phase:** 46-docs-currency-hygiene
**Areas discussed:** Migration guide shape & freeze, Treasurer page & config reference, Nyquist
promotion depth, v2 debt lines: fix or waive, plus two closing questions (removed-keys todo shape,
paladin-eval link date)

**Session notes.** The session started on the 2026-10-20 tree (`8ad298dd`, Phase 50.1 paused at
plan 07, Phase 51 unplanned). Mid-session the owner asked for the branch to be pulled; the
fast-forward to `e28e43b5` (147 commits) brought Phases 50.1 and 51 sealed and STATE pointing at
Phase 46. The owner then supplied web3sec's dogfood report, saved verbatim as
`46-FEEDBACK-web3sec.md`; the "freeze timing" question was re-asked against the new state.

---

## Todos matched to this phase

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 44 review follow-ups | WR-04 docs + one-time warn, WR-05 dated semver-allow note, IN-* items | ✓ |
| Reject removed v0.10 config keys | Option B load-time error naming key + Aegis replacement | ✓ |
| web3sec single bump to v0.11.0 | Folded as an input (divergence report feeds the guide) | ✓ |
| Verify local coverage reproduction | Needs a Docker machine; maintainer-walked | |

(Two further matches, the content/data-orchestration review and the OpenAI Responses API todo,
were listed as reviewed-not-folded.)

---

## Migration guide shape & freeze

| Option | Description | Selected |
|--------|-------------|----------|
| New v0.11 part on top, v0.10 kept below | §10 above §9; interleaved v0.11 rows move into §10 | ✓ |
| Separate file per release | Move v0.10 record to a docs/migrations file | |
| Leave rows, add consumer guide page only | Narrative guide in the book, register unchanged | |

| Option | Description | Selected |
|--------|-------------|----------|
| Freeze in Phase 46 on this report | Fold G-1..G-7; depends-on clause satisfied | ✓ |
| Leave a final-touch plan for Phase 47 | Release checklist re-confirms §10 against the tag | |

| Option | Description | Selected |
|--------|-------------|----------|
| Move rows, keep IDs, leave a pointer | Allowlist keys unchanged; historical citations untouched | ✓ |
| Move rows and rewrite every live citation | Sweep ADRs/CHANGELOG/rustdoc to §10.x | |
| Copy rather than move | Duplicate rows in §10 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Rewrite upgrading.md for v0.11, keep v0.10 beneath | One page, one pointer line in migration-guide.md | ✓ |
| New page per release | upgrading-v0.11.md + index | |
| Pointer only | upgrading.md points at MIGRATION.md §10 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Fold into Phase 46 as a small code item | One-time warn! on uncomposed 429 | |
| Defer: G-7 wording is enough for v0.11.0 | Record as deferred idea | ✓ |

**User's choice:** all recommended options. **Notes:** a first version of the freeze-timing
question (two waves vs wait vs patch in 47) was withdrawn before answer when the owner asked for
the pull; it became moot once 50.1 and 51 were sealed.

---

## Treasurer page & config reference

| Option | Description | Selected |
|--------|-------------|----------|
| Architecture, beside Commissary | `architecture/treasurer.md` concept page | ✓ |
| User Guides | Task-first `user-guides/treasurer.md` | |
| Operations | Deploy-and-run page beside Observability | |

| Option | Description | Selected |
|--------|-------------|----------|
| One section on the Treasurer page | "Rate pacing (Cadence)" → ADR-0058 + config table | ✓ |
| Own page beside it | `architecture/cadence.md` | |
| Leave Cadence as it is | configuration.md only | |

| Option | Description | Selected |
|--------|-------------|----------|
| Extend configuration.md into one Treasurer reference block | Add Pricing, regroup under `## Treasurer` | ✓ |
| New reference page | `api-reference/treasurer-configuration.md` | |

| Option | Description | Selected |
|--------|-------------|----------|
| Rust structs + config.example.yml, every key tabled | `src/config/treasurer.rs` is the truth | ✓ |
| Document the common path only | Pricing, ceilings, warn_at only | |

**User's choice:** all recommended options. **Notes:** none.

---

## Nyquist promotion depth

| Option | Description | Selected |
|--------|-------------|----------|
| Reconcile against existing tests, record gaps honestly | Auditor fills the map; no new tests | ✓ |
| Reconcile and write the missing tests | All seven reach compliant | |
| Status flip with a dated note only | No per-task map | |

| Option | Description | Selected |
|--------|-------------|----------|
| Promote to compliant, citing the recorded sign-offs | 28-UAT 2/2, D-37, Phase 45 re-measure | ✓ |
| Keep non-compliant, record the gap | Overhead bar miss as explicit gap | |

| Option | Description | Selected |
|--------|-------------|----------|
| Edit in place in the archive | Resolver finds archived phases | ✓ |
| Copy into Phase 46 and link back | `46-NYQUIST-<N>.md` files | |

| Option | Description | Selected |
|--------|-------------|----------|
| /gsd-validate-phase N per phase, as plan tasks | Eight invocations | ✓ |
| One hand-written audit pass | Executor fills by hand | |

**User's choice:** all recommended options. **Notes:** none.

---

## v2 debt lines: fix or waive

| Option | Description | Selected |
|--------|-------------|----------|
| Waive with a measured re-statement | Re-measure, defer as next-milestone candidate | ✓ |
| Fix: split the two engine files | Decompose superstep.rs / engine/mod.rs now | |
| Fix the original three only | Honour the letter of the line | |

| Option | Description | Selected |
|--------|-------------|----------|
| Waive: the blocker is gone, no measured hotspot | Benches exist; nothing to optimise against | ✓ |
| Measure first, then decide | Add a targeted bench in Phase 46 | |

| Option | Description | Selected |
|--------|-------------|----------|
| Record as fixed by ADR-0015, add a mechanical guard | Script/test wired into check-gates | ✓ |
| Record as fixed by ADR-0015, no guard | Waiver text only | |
| Re-measure and amend ADR-0015 | Count again, amend, then guard | |

| Option | Description | Selected |
|--------|-------------|----------|
| WINDOWS.md rows via the ledger tool | One row per line, STATE table points at ids | ✓ |
| A single ADR | ADR-0063 disposition record | |
| STATE.md deferred table only | Rewrite the three rows | |

**User's choice:** all recommended options. **Notes:** the guard decision was recorded with the
rider that a moved dependency count amends ADR-0015 first (CONTEXT D-21).

---

## Closing questions

| Option | Description | Selected |
|--------|-------------|----------|
| I'm ready for context | Write CONTEXT.md | ✓ |
| Explore more gray areas | Removed-keys shape, paladin-eval link date | |

| Option | Description | Selected |
|--------|-------------|----------|
| Confirm B in Phase 46, scoped as the todo sketches | Per-key rejection, ADR-0059 amended | ✓ |
| Switch to A: callout in the guide only | No code | |
| Defer B to Phase 47 | Pre-tag item | |

| Option | Description | Selected |
|--------|-------------|----------|
| Record 'before 2026-09-21 (proven by run 35659477719)' | Evidence-class-honest cell | ✓ |
| I'll supply the exact date | Reported date | |

---

## Claude's Discretion

§10 prose and consumer-index grouping; the Treasurer page's worked example; the guard's form
(script vs cargo test); plan slicing and wave order; confirming the 50.1 WR-01..03 fixes ride the
§9.8 → §10.8 move; the optional Phase 44 info items.

## Deferred Ideas

One-time `warn!` on an uncomposed 429 (web3sec §4); oversized-file decomposition as a
next-milestone candidate phase; Nyquist reconciliation for archived Phases 05–21; optional Phase 44
test-tightening items.
