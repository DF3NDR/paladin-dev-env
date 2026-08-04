# ADR Conventions and Promotion

This file is the small shared index that every phase appending to `.planning/decisions/` reads
before writing an ADR. It answers four questions the numbering scheme, the required headings, and
the supersession mechanism raise once — so Phases 5, 7, 10 and 13 do not have to re-derive them.

## Numbering scheme

ADR files use a **flat, zero-padded, monotonic counter**: `NNNN-kebab-slug.md`.

Chosen over a phase-scoped prefix (`p01-…`, `p05-…`) because a phase prefix breaks the moment an
ADR is superseded by a *later* phase's ADR — the reader would have to know which phase number is
"newer" rather than just comparing the counter. A flat counter surviving Phases 1, 5, 7, 10 and 13
needs only one shared piece of state: the next free number, tracked below.

## Numbering index

Reserved for Phase 1 (this phase authors ADR-0005 only; 0001-0004 and 0006 are reserved slots for
the plans that follow in this same phase):

| Number | Slug | Subject |
|---|---|---|
| 0001 | `battalion-config` | `BattalionConfig` field set (RECON-02) |
| 0002 | `battalion-result` | `BattalionResult` field set (RECON-03) |
| 0003 | `formation-min-paladins` | Formation minimum Paladin count (RECON-04) |
| 0004 | `temperature-validation` | Provider-aware temperature range (RECON-05) |
| 0005 | `herald-trait` | `Herald` trait signature (RECON-06) |
| 0006 | `coverage-gate` | Project-wide test coverage gate (RECON-07) |
| 0007 | `battalion-cancellation-deferral` | Battalion-wide cancellation deferred to Phalanx-only, per D-05/D-08 (Phase 2) |
| 0008 | `workspace-version-0-7-0` | Workspace version converges on 0.7.0, per D-01/D-02 (Phase 4) |
| 0009 | `workspace-rust-edition-2024` | Workspace Rust edition converges on 2024, per D-04/D-06 (Phase 4) |
| 0010 | `prompt-context-budgeting` | Prompt/context budgeting against declared `max_context_tokens` (LLMR-04, Phase 41) |

**Next free ADR number: 0011**

Phases 5, 7, 10 and 13 take the next free number from this line when they author further ADRs —
they do not need to `ls` the directory to find it. Each phase updates this line when it appends.

## Required heading set

Every ADR uses the following H2 headings, in this order:

- `## Status`
- `## Context`
- `## Decision`
- `## Considered Options`
- `## Code Locations`
- `## Code Conformance`
- `## Downstream Consumers`

`## Code Locations` and `## Considered Options` are **bulleted lists, never prose paragraphs** —
`.claude/gsd-core/bin/lib/adr-parser.cjs`'s `splitEntries` only yields structured entries from
bullet or numbered lines; a paragraph collapses into one opaque blob and defeats the whole point of
citable, checkable entries.

`## Code Conformance` and `## Downstream Consumers` have no synonym in `adr-parser.cjs`'s
`CANONICAL_HEADERS` table and land in the parser's `unmapped_headers` bucket. That is acceptable —
nothing currently consumes either field programmatically — but they are still required, since
`## Code Conformance` is D-03's contract (every ADR MUST carry a `conforms` / `must change` verdict)
and `## Downstream Consumers` names who reads the decision next.

## Supersession mechanism

Exactly one live ADR answers each question at any time. When a later ADR supersedes an earlier one:

- The **superseded ADR keeps its file** — it is never deleted or renamed.
- Its `## Status` body becomes the bare word `Superseded`, followed by a prose line naming the
  superseding ADR's number and the reason it no longer holds.
- The **superseding ADR** carries a `## Supersedes` line naming the ADR number it replaces.
- `adr-parser.cjs` recognises `superseded` as a status word (see `STATUS_REJECT_SET` /
  `parseStatusFromSections`), so a downstream consumer can mechanically tell a live ADR from a
  retired one without reading prose.

## Promotion procedure for existing ADR candidates

**Phase 1 promotes none of the eleven existing ADR candidates.** Each candidate stays with its
owning phase, listed in the inventory below — Phase 1 builds the mechanism (this file, and the
worked example at `.planning/decisions/0005-herald-trait.md`) but does not use it on any of the
eleven itself.

### Part A — the procedure

An owning phase promotes one of its candidates into `.planning/decisions/` by:

1. Taking the next free number from the **Numbering index** line above and decrementing nothing —
   numbers are never reused, even if a candidate is later rejected instead of accepted.
2. Authoring the candidate's substance into the standard heading set (`## Status` · `## Context` ·
   `## Decision` · `## Considered Options` · `## Code Locations` · `## Code Conformance` ·
   `## Downstream Consumers`), following `0005-herald-trait.md`'s shape.
3. Setting `## Code Conformance` to `conforms` or `must change` per D-03 — naming the executing
   requirement (e.g. a `GAP-*`, `ARCH-*`, or phase-specific ID) where the verdict is `must change`.
4. Citing the source document's path in `## Code Locations` alongside the shipped-code citations,
   so a reader can trace the promoted decision back to the corpus document it came from.
5. Updating the `Next free ADR number` line in this file.
6. Adding a row to `.planning/PROJECT.md`'s `## Key Decisions` table, linking to the new ADR file.

**Why this is viable now, where it previously was not.** Before this phase, promoting a candidate
required re-tagging its source document via `--manifest` and re-running the ingest classifier —
and the ingest is closed (STATE.md: "there is no run 6"). That path no longer exists. It is not
needed either: ADRs now live in `.planning/decisions/` as their own document class, independent of
the ingest manifest, and top the precedence order (D-01, D-02). Promotion is now an ordinary write
to a directory plus a table row — the same six steps any of Phase 1's six ADRs already followed.

### Part B — the inventory

One entry per candidate. Each carries the source document path, the ingest run that surfaced it,
what it decides in one line, and an explicit **Owner phase**.

1. **`Milestone_5/Epic_1/decisions/battalion-result-upward-dependency-decision.md`** (run 3) —
   settles where `PaladinResult`, `StopReason`, `TokenUsage`, `RegistryError` and `HandoffError`
   live (`paladin-core`); shipped code implements it, but a later PRD outranks it on paper.
   **Owner phase: Phase 7.**
2. **`Epic_17.5/epic17-5.md`** (run 2) — the CLI belongs in `src/application/cli` as an input
   adapter, not infrastructure; already applied in code, also outranked by a PRD that says
   otherwise. **Owner phase: Phase 7.**
3. **`Milestone_7/Epic_4/rustsec-remediation-plan.md`** (run 4) — formal risk acceptance of two
   RustSec advisories, owner Platform Security, **review/expiry target 2026-09-30 — the only dated
   item in the entire 263-document corpus**. **Owner phase: Phase 9.**
4. **`Milestone_7/Epic_1/cost-benefit-assessment.md`** (run 4) — go/defer scoring for four
   candidate crate extractions, with a named approver and a Self-Approval block. **Owner phase:
   Phase 10.**
5. **`Milestone_7/Epic_4/license-compatibility-decision-checklist.md`** (run 4) — licensing policy
   accepting MPL-2.0 with a 551-package inventory, approver `DF3NDR`. **Owner phase: Phase 10.**
6. **`Milestone_8/facade-cleanup-RECONCILIATION-2026-06-04.md`** (run 4) — supersession notice that
   corrected two prior documents and resolved six open decisions in execution. **Owner phase:
   Phase 10.**
7. **`Milestone_10/Epic_2/prd-dependency-security-license-compliance.md` FR-1 + §8** (run 5) — the
   audit-suppression single-source invariant (exceptions live only in `audit.toml` and
   `deny.toml`), currently violated by the tree. **Owner phase: Phase 12.**
8. **`Milestone_12/Epic_1/prd-agent-registry-execution-api.md` §7 + OQ-2** (run 5) — the
   `AgentProvisioner` placement, currently recorded as a default rather than a decision.
   **Owner phase: Phase 13.**
9. **`Milestone_9/Epic_4/prd-agent-orchestrator-bridge.md` §6.1** (run 5) — the bidirectional
   content/agent bridge decision; the cleanest ADR-shaped section anywhere in the corpus (a
   four-criterion comparison table with a `(CHOSEN)` column). No open forward requirement currently
   points at it, so this owner assignment is Claude's Discretion rather than a CONTEXT.md-recorded
   answer: grouped with candidate 8 under the same Milestone 9-12 close-out phase, since both are
   run-5 Milestone 9/12 subjects. **Owner phase: Phase 13.**
10. **`Milestone_9/Epic_5/prd-user-admin-system-completion.md` §6.1** (run 5) — the
    opaque-bearer-token decision; the only decision in the corpus a later milestone contradicts in
    prose while silently preserving in code. **Owner phase: Phase 14.**
11. **`Deferred-QA-CICD-Completion/DEFERRED_COVERAGE.md`** (run 5) — the coverage deferral record,
    with a named sign-off and an unreached "Next Review" trigger; weaker than the others, since its
    two module paths are stale and its baselines predate Milestone 9. **Owner phase: Phase 15.**
