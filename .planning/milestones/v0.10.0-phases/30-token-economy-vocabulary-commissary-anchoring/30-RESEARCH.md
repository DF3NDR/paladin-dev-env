# Phase 30: Token-Economy Vocabulary & Commissary Anchoring - Research

**Researched:** 2026-09-14
**Domain:** Documentation / ADR authoring (docs-only, non-breaking). No Rust code behavior changes;
only rustdoc/comment text edits.
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Vocabulary rule (D-1 → VOCAB-01)**
- The rule is: **units/measures and technical ports keep plain industry names; domain roles,
  places and events get Medieval-Military names.** Plain: `TokenUsage`, `max_tokens`,
  `max_context_tokens`, `TokenBudget`, `TokenCounterPort`, `LlmPort`, `EmbeddingPort`,
  `token_budget.*`. Medieval: Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest,
  Commissary, Treasurer (reserved).
- Write the rule into `.planning/PROJECT.md` (the ubiquitous-language material) AND
  `docs/src/architecture/domain-model.md`. The `CLAUDE.md` / `.github/copilot-instructions.md`
  ubiquitous-language table is the developer-facing list — add `Commissary` there too so the
  three lists agree (planner's discretion on exact placement; the lists must not disagree).
- `Commissary` is framed everywhere as the **input-side, per-call window-rationing officer** —
  "what fits in this sortie's pack": `verify_fits` guard + `dispense` allocator over a
  `Consignment` → `Stockpile` + `ShedItem`s, fail-loud / never-silent.

**Commissary anchoring (D-2 → VOCAB-02, VOCAB-03)**
- One numbered ADR in `.planning/decisions/` (next free number; follow `PROMOTION.md`'s numbering
  scheme and the heading shape of `0048-paladin-eval-composition-crate.md`) recording: the
  `Commissary` design (`verify_fits` guard + `dispense` allocator, fail-loud / never-silent —
  the ADR-0010 refusal semantics), the Quartermaster→Commissary rename rationale, and the
  **explicit rejected-name list**. Reconstruct from
  `origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`
  (168 lines, `git show` it — do not check the branch out) and the port commits `348f5910`
  (`feat(paladin-llm): port v0.10.0-native prompt-budgeting Commissary service`) and `35fd8390`.
  Cite ADR-0010 by its abandoned-branch path; do not copy it wholesale as a live ADR — the new
  ADR is the on-branch record.
- One mdBook page for `Commissary` under `docs/src/` (planner picks the path — the natural home
  is the architecture section or a concepts page next to the domain model): concept, the
  `Consignment` / `ConsignmentItem` / `DispensedItem` / `Stockpile` / `ShedItem` /
  `CommissaryPlan` / `CommissaryError` model, and a usage sketch. Link it from
  `docs/src/SUMMARY.md` so it is reachable from the architecture nav. The docs CI
  (`.github/workflows/docs.yml`: mdbook 0.4.40, mdbook-mermaid 0.13.0, mdbook-linkcheck 0.7.7,
  `warning-policy = "error"`) must stay green — every intra-book link must resolve.
- The usage sketch must be a real, compiling shape (a doc test or an `ignore`-marked block that
  mirrors an existing unit test in `commissary.rs`), never invented API.

**Treasurer reservation (D-3 → VOCAB-04)**
- One one-page **reservation** ADR (separate from the Commissary ADR). It states: `Treasurer` is
  reserved (0/0 in-tree, verified by `grep -rn Treasurer crates src docs` at authoring time); it
  will own cross-run / per-tenant / per-API-key **allowances**, per-model currency **pricing**,
  `cost_estimate` production, and rate **pacing**; it **installs** a per-run `TokenBudget`
  (the existing `src/application/services/paladin/middleware/limits.rs` mechanism) rather than
  replacing it; it is **built in Milestone 14** (`.project/Milestone_14-Treasurer/`, deferred,
  hard-depends on Phase 31), not this cycle.
- The **downstream guardrail** is part of the ADR text: `Treasurer` is a framework-only word; in
  the downstream Web3 Security Paladin app it collides with a benchmark fixture
  (`GarrisonTreasury`, an audit-*target* domain term), so the framework term must never be used
  as an audit-target or fixture domain term, and vice versa.
- Record the rejected alternatives: `Paymaster` (rejected — a paymaster pays the troops; tokens
  are spent on the provider), `Comptroller` (collision-free alternative; operator chose
  `Treasurer` and accepted the guardrail). The two-officer model (Commissary = input-side
  per-call rationing; Treasurer = output-side cross-run spend governance) is **operator-confirmed
  2026-09-14** and locked.

**`max_tokens` disambiguation and `cost_estimate` reservation (D-8, D-9 → VOCAB-05)**
- Add ONE table to `docs/src/getting-started/configuration.md` naming the four `max_tokens`
  meanings: (1) Garrison store cap, (2) RAG injection cap (`rag.max_tokens`), (3) per-request
  completion cap, (4) run-level `token_budget` cap — each row naming the config key / type that
  owns it (planner: verify each against the tree, e.g. `grep -rn max_tokens crates src docs`).
  State that any future Treasurer-level cap uses a distinct key (`allowance`), never `max_tokens`.
- Update the rustdoc on `ExecutionMetadata.cost_estimate`
  (`crates/paladin-core/src/platform/container/herald.rs`, field ~line 521, builder ~line 607,
  doc ~line 387 and the example ~line 471 that says "$0.045 based on GPT-4 pricing") to say the
  field is **reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet.** Do
  **not** remove the field, the accessor or the builder method — a rustdoc-only change, no
  `MIGRATION.md` §9.2 entry, no semver-checks allowlist change.

**Quartermaster purge (VOCAB-06)**
- `src/lib.rs:195` provenance comment (`// v0.10.0-native re-port of the removed
  Quartermaster/Convoy/apportion capability`) — reword to describe the Commissary port without
  the retired term, or drop it.
- `.project/project-management/paladin-project-plan-final.md` `name: "SirQuartermaster"` example
  — annotate as historical (a one-line note; do not rewrite the document).
- Exit check: `grep -rniE '\bQuartermaster\b' crates src` returns nothing. `.planning/` phase
  history (Phase 26 plans that record the Quartermaster→Commissary port) is **untouched** —
  overview §5.4 forbids rewriting planning history. The Milestone 13 overview/PRDs under
  `.project/Milestone_13-Token-Economy/` legitimately mention the retired term and are out of the
  grep's scope.

**Versioning decision ADR (VOCAB-07 — roadmap-time addition, not in the PRD)**
- One ADR recording that **Phases 31-33 land as clean breaks inside the untagged v0.10.0**:
  the v0.10.0 corpus rule X-03 (`.project/v0.10.0/00-program-overview.md:44` — "Deprecations are
  allowed with `#[deprecated]` but removals are not") is **superseded for Phases 31-33 only** by
  the Milestone 13 overview §5.1 clean-break policy, on the operator's 2026-09-14 decision
  (pre-1.0; one coordinated downstream consumer that pins its submodule pointer and adopts the
  whole milestone at once). Every break still gets a `MIGRATION.md` §9.2 row and a
  `cargo semver-checks` allowlist row (Phase 29 D-04 row-level gate) **as documentation for the
  downstream refactor, never as a compatibility shim**; the `0.10.0` tag is cut only after
  Phase 33 re-seals the Phase 29 release gates (COMM-04).
- Add a matching row to `.planning/PROJECT.md` **Key Decisions** linking the ADR (the table
  links ADRs rather than restating them).

**Locked by the milestone overview §0 (do not revisit)**
- `Commissary` — keep, do not rename. `TokenBudget`, `TokenCounterPort`, `TokenUsage`,
  `max_tokens`, `token_budget.*` — do not rename. `Quartermaster` — stays retired, never
  reintroduced. `Paymaster` — rejected. `Treasurer` — the reserved term.
- Version identity: this work ships in **v0.10.0** (the PRD's "v0.11.0" target is superseded by
  the operator's instruction; `0.10.0` is bumped on the feature branch with no tag — Phase 29
  D-18/D-21). Write "v0.10.0" in every new doc, never "v0.11.0".
- Rustdoc references to the Treasurer point at **Milestone 14**, not "Epic 5" (the PRD's R7
  wording predates the Milestone 14 split).

### Claude's Discretion
- ADR numbers (next free after 0048), titles and file names; whether the Commissary ADR carries
  the rejected-name list inline or in an appendix section.
- mdBook page path/name and where in `SUMMARY.md` it sits (must be under the architecture nav or
  linked from it).
- Exact wording of the vocabulary rule and of the historical annotation.
- Whether the `CLAUDE.md` / `copilot-instructions.md` ubiquitous-language table gains a
  `Commissary` row now (recommended, one row) — it must not contradict the new rule.
- How to verify docs build locally (`mdbook build docs/` if the pinned tools are installed,
  otherwise rely on the CI job and a link grep).

### Deferred Ideas (OUT OF SCOPE)
- Building the Treasurer (pricing, `cost_estimate` producer, allowances, pacing) — Milestone 14,
  `.project/Milestone_14-Treasurer/`; FUT-08 / FUT-09 stay v2.
- Any code-behaviour change, any rename of shipped types, `TokenUsage` carrier changes
  (Phase 31), `TokenCounterPort::is_exact` / `Commissary::new` signature / legacy counter
  removal / shared window resolver (Phase 32), RAG adoption of `Commissary` (Phase 33).
- Rewriting `.planning/` phase history that mentions Quartermaster — forbidden (overview §5.4).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VOCAB-01 | Vocabulary rule written into `PROJECT.md` and `docs/src/architecture/domain-model.md`; `Commissary` in both the ubiquitous-language list and the domain-model table as the input-side, per-call window-rationing officer | Pattern 4 identifies the exact three lists and their current (verified) state, line numbers, and what's missing from each |
| VOCAB-02 | Numbered ADR records the `Commissary` design, rename rationale, and rejected-name list, reconstructed from ADR-0010 and the port commits | Pattern 1 (ADR authoring procedure, next number = 0049) + Pattern 2 (full verified source material from ADR-0010 and commits `348f5910`/`35fd8390`, combined rejected-name list) |
| VOCAB-03 | mdBook page for `Commissary` linked from `SUMMARY.md`, link-check green | Standard Stack (tool versions verified installed) + Pattern 3 (usage-sketch source material, mirrored from real tests) |
| VOCAB-04 | One-page `Treasurer` reservation ADR: 0/0 in-tree, ownership scope, installs not replaces `TokenBudget`, built in Milestone 14, downstream guardrail | Pattern 1 (ADR-0050) + Milestone 14 overview/PRD full reads (Treasurer scope, hard prerequisite, guardrail text) |
| VOCAB-05 | `max_tokens` four-meanings table + `cost_estimate` rustdoc reservation note, field not removed | Architecture Patterns diagram (four config keys located exactly) + Pitfall 2/3 (precise wording guidance) + exact `herald.rs` line citations (387, 471, 521, 551, 607) |
| VOCAB-06 | `grep -rniE '\bQuartermaster\b' crates src` returns nothing; `src/lib.rs` comment reworded; plan-final doc annotated; `.planning/` untouched | Runtime State Inventory (confirms single hit, non-functional) + exact citations (`src/lib.rs:195`, `paladin-project-plan-final.md:1112`) |
| VOCAB-07 | Versioning ADR records Phases 31-33 clean-break supersession of X-03; `MIGRATION.md`/allowlist rows as documentation; `PROJECT.md` Key Decisions row | Pattern 1 (ADR-0051) + exact X-03 citation (`00-program-overview.md:44`) + `PROJECT.md:355` (explicit forward pointer to this ADR) |

</phase_requirements>

## Summary

Phase 30 is a pure documentation phase: three new ADRs, one new mdBook page, two doc-table edits,
one rustdoc edit, two comment/prose edits, and two vocabulary-list edits. Every fact this phase
needs to state (the `Commissary` service's real API surface, the abandoned-branch ADR-0010's design
record, the port commits' rejected-name list, the `cost_estimate` field's exact rustdoc lines, the
four `max_tokens` config keys, the 0/0 `Treasurer` grep, and the two orphan `Quartermaster`
references) was independently verified in this session directly against the tree — none of it is
inherited unverified from the CONTEXT.md or the milestone corpus.

**Primary recommendation:** Treat this as a documentation-writing phase with a verification-by-grep
exit gate, not a code phase. The planner should structure work as: (1) one ADR for Commissary
(design + rename rationale + rejected names, ADR-0049), (2) one ADR for the Treasurer reservation
(ADR-0050), (3) one ADR for the versioning/X-03 supersession (ADR-0051), (4) one new mdBook page
plus `SUMMARY.md` link, (5) `PROJECT.md` + `docs/src/architecture/domain-model.md` +
`CLAUDE.md`/`copilot-instructions.md` vocabulary-list edits, (6) the `configuration.md` `max_tokens`
table, (7) the `herald.rs` rustdoc edit, (8) the two Quartermaster purge edits. `mdbook build docs/`
is runnable locally (all three pinned tools are installed at the CI-pinned versions) and should be
the primary verification command, backed by grep-based checks for the exit criteria that are pure
text assertions (`Quartermaster`, `Treasurer` absence).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Vocabulary rule (D-1) | Docs/Planning record | — | `PROJECT.md` (planning corpus) + `docs/src/architecture/domain-model.md` (published docs) + `CLAUDE.md`/`copilot-instructions.md` (developer-facing source-of-truth table) — three lists, no code |
| Commissary ADR + rejected-name record | Docs/Planning record | — | `.planning/decisions/` is a planning-record artifact, not shipped code; it documents an existing `paladin-llm` domain service, it does not modify it |
| Commissary mdBook page | Docs (published) | — | `docs/src/` under the `mdbook`/`mdbook-linkcheck` toolchain; content describes `crates/paladin-llm/src/services/commissary.rs`'s real, already-shipped public API |
| Treasurer reservation ADR | Docs/Planning record | — | Reserves a term; institutes no code, no crate, no port |
| `max_tokens` disambiguation table | Docs (published) | Config/Backend (source of truth being documented) | The table documents four *existing* config keys (`garrison.max_tokens`, `rag.max_tokens`, the per-request `LlmRequest.metadata["max_tokens"]`/adapter-config key, `agent_runtime.token_budget.max_tokens`) that already live in `src/config/` and `crates/paladin-llm/src/*/adapter.rs`; the doc table follows the code, the code does not change |
| `cost_estimate` rustdoc reservation note | Docs (rustdoc, published via `cargo doc`) | Core domain (`paladin-core`) | Field/accessor/builder text lives in `crates/paladin-core/src/platform/container/herald.rs`; edited in place, no signature change |
| Quartermaster purge | Docs/Comments | — | `src/lib.rs:195` is a Rust comment (not rustdoc, not compiled documentation) inside the facade crate; the plan-final doc is a `.project/` planning artifact |
| Versioning/X-03 supersession ADR | Docs/Planning record | Program gates (Phase 29/33 CI) | Records a policy exception for Phases 31-33's *future* code changes; this phase writes the record only, touches no CI config, no `MIGRATION.md` row (Phase 30 itself makes no public-API change) |

## Package Legitimacy Audit

Not applicable — this phase installs no new dependencies (no `Cargo.toml` changes of any kind).

## Standard Stack

No new libraries. The phase uses only the already-pinned documentation toolchain:

| Tool | Version (pinned in `docs/book.toml` / `.github/workflows/docs.yml`) | Locally installed? | Purpose |
|------|------|------|---------|
| `mdbook` | 0.4.40 | ✓ verified (`mdbook --version` → `mdbook v0.4.40`) | Builds `docs/` |
| `mdbook-mermaid` | 0.13.0 | ✓ verified | Mermaid diagram preprocessor (usable for the Commissary flow diagram) |
| `mdbook-linkcheck` | 0.7.7 | ✓ verified | `warning-policy = "error"` — every intra-book link must resolve |

**Installation:** none required — all three binaries are already on `PATH` at the exact CI-pinned
versions `[VERIFIED: local shell — mdbook --version / mdbook-mermaid --version / mdbook-linkcheck --version]`.
This means `mdbook build docs/` can be the planner's real, fast local verify command rather than a
CI-only fallback — CONTEXT.md leaves this to Claude's discretion; the answer is: use it directly.

## Architecture Patterns

### System Architecture Diagram

```
                 ┌─────────────────────────────────────────────────────────┐
                 │           .planning/decisions/ (ADR record)              │
                 │                                                           │
  PROMOTION.md   │  0049-commissary-*.md      (design + rename + rejected)   │
  (numbering,    │  0050-treasurer-*.md       (reservation + guardrail)      │
  headings,      │  0051-token-economy-*.md   (X-03 supersession for 31-33)  │
  supersession)  │                                                           │
                 └───────────────┬───────────────────────────────────────────┘
                                 │ each links from
                                 ▼
                 ┌─────────────────────────────────────────────────────────┐
                 │        .planning/PROJECT.md  Key Decisions table          │
                 │  (rows link ADR files, never restate them — PROMOTION.md  │
                 │   Part A step 6)                                          │
                 └─────────────────────────────────────────────────────────┘

  Vocabulary rule (D-1) fans out into THREE published/planning lists that
  must agree (none is auto-generated from another):

  .planning/PROJECT.md              docs/src/architecture/          CLAUDE.md +
  ## Constraints                    domain-model.md                 .github/copilot-instructions.md
  "Ubiquitous language" bullet  ──▶ Medieval Military Naming     ──▶ Naming Convention table
  (line ~1236)                      Convention table (add row)       (developer-facing source)
        │                                 │                                │
        └───────── all three gain a "Commissary" row/clause ───────────────┘
              (Treasurer is named in prose as "reserved", not a live row)

  Commissary mdBook page:
  docs/src/SUMMARY.md (Architecture section, new line)
        │
        ▼
  docs/src/architecture/commissary.md (new page)
        │  describes, does not modify:
        ▼
  crates/paladin-llm/src/services/commissary.rs
  (Commissary::verify_fits guard, Commissary::dispense allocator,
   Consignment/ConsignmentItem → Stockpile/DispensedItem/ShedItem)

  max_tokens disambiguation table:
  docs/src/getting-started/configuration.md
        │  documents FOUR existing, independent config keys:
        ▼
  ┌────────────────────┬──────────────────────────────┬─────────────────────────┐
  │ garrison.max_tokens │ rag.max_tokens                │ per-request completion   │ run-level
  │ (Garrison store cap)│ (RAG injection cap)           │ cap (LlmRequest metadata │ token_budget cap
  │ src/config/*        │ docs/src/getting-started/     │ "max_tokens" / adapter   │ agent_runtime.
  │                      │ configuration.md:283          │ config.max_tokens)       │ token_budget.max_tokens
  └────────────────────┴──────────────────────────────┴─────────────────────────┴──────────────────

  cost_estimate rustdoc reservation:
  crates/paladin-core/src/platform/container/herald.rs
  (field ~521, accessor total_cost() ~551, builder ~607, doc-comment list ~387, example ~471)
        │ read-only consumers, unaffected by the doc edit:
        ▼
  crates/paladin-herald/src/json_herald.rs:208 , markdown_herald.rs:343
  (both read metadata.cost_estimate — currently always None; no producer anywhere in the tree)
```

### Recommended Plan Structure

Given this is docs-only with a hard "no code behavior change" boundary, structure plans/waves by
artifact rather than by requirement-cluster tier:

```
plan A — ADRs (VOCAB-02, VOCAB-04, VOCAB-07)
  - ADR-0049 Commissary design + rename rationale + rejected-name list
  - ADR-0050 Treasurer reservation + downstream guardrail
  - ADR-0051 Token-economy versioning / X-03 supersession for Phases 31-33
  - PROJECT.md Key Decisions: three new rows linking the three ADRs
  - PROJECT.md ## Constraints "Ubiquitous language" bullet: add Commissary
    (Treasurer stays out of the bullet's enumerated list — it is reserved,
    not a live term with a shipped type; name it in nearby prose instead,
    matching how PROJECT.md:344-348 already does today)

plan B — Vocabulary + domain-model + mdBook page (VOCAB-01, VOCAB-03)
  - docs/src/architecture/domain-model.md: add the vocabulary-rule prose
    + a Commissary row in the "Medieval Military Naming Convention" table
  - CLAUDE.md + .github/copilot-instructions.md: add Commissary row to the
    "Naming Convention" table (the two files are @-imported into CLAUDE.md,
    but copilot-instructions.md is the actual source table — edit it there;
    CLAUDE.md's own prose paragraph, if any, should stay consistent)
  - docs/src/architecture/commissary.md (new page): concept, the
    Consignment → dispense → Stockpile/ShedItem model, a usage sketch
    mirrored from commissary.rs's own #[cfg(test)] tests (never invented API)
  - docs/src/SUMMARY.md: new line under "# Architecture"

plan C — max_tokens table + cost_estimate rustdoc (VOCAB-05)
  - docs/src/getting-started/configuration.md: one new table, 4 rows +
    the allowance-key sentence
  - crates/paladin-core/src/platform/container/herald.rs: reword the
    cost_estimate field doc (~521), the doc-comment bullet (~387), and the
    "$0.045 based on GPT-4 pricing" example comment (~471) to say reserved
    for Treasurer (Milestone 14 / FUT-08), no in-tree producer

plan D — Quartermaster purge (VOCAB-06)
  - src/lib.rs:195 comment reword (drop "Quartermaster/Convoy/apportion")
  - .project/project-management/paladin-project-plan-final.md: one-line
    historical annotation above/near the SirQuartermaster example (line 1112)
  - exit grep: `grep -rniE '\bQuartermaster\b' crates src` → empty

plan E (or folded into A) — verification
  - `mdbook build docs/` green (linkcheck warning-policy=error)
  - `grep -rn Treasurer crates src` → empty (still true after plan A/B —
    the ADR text itself lives under .planning/decisions/, NOT under
    crates/src, so it does not trip this grep)
  - `cargo doc -p paladin-core` clean (no new intra-doc-link warnings)
```

### Pattern 1: ADR authoring via PROMOTION.md's procedure
**What:** `.planning/decisions/PROMOTION.md` is the one shared index every phase reads before
writing an ADR. Its "Part A — the procedure" (6 steps) is the actual authoring checklist.
**When to use:** For all three of this phase's ADRs.
**Steps, concretely for Phase 30:**
1. Next free number is **0049** `[VERIFIED: .planning/decisions/PROMOTION.md:70 "Next free ADR
   number: 0049"]`. Commissary → 0049, Treasurer → 0050, versioning → 0051 (three ADRs, +3, matching
   the precedent multi-ADR-per-phase notes already in PROMOTION.md for Phases 9, 10, 13, 14, 17).
2. Required heading set, in order (`## Status`, `## Context`, `## Decision`, `## Considered
   Options`, `## Code Locations`, `## Code Conformance`, `## Downstream Consumers`)
   `[VERIFIED: PROMOTION.md:212-220]`. `## Code Locations` and `## Considered Options` MUST be
   bulleted lists — `adr-parser.cjs`'s `splitEntries` only yields structured entries from bullet/
   numbered lines `[VERIFIED: PROMOTION.md:222-225]`.
3. `## Code Conformance` carries `conforms` or `must change` (D-03's contract). All three of this
   phase's ADRs should read **`conforms`** — none of them instructs a code change; they record
   already-shipped state (Commissary), a term reservation (Treasurer), or a forward-looking policy
   exception for later phases (versioning) `[VERIFIED: PROMOTION.md:261-262]`. Precedent: ADR-0048
   used `conforms` for exactly this "records the classification, instructs no code change" shape
   `[VERIFIED: 0048-paladin-eval-composition-crate.md:126-134]`.
4. Cite the source document path (ADR-0010 on the abandoned branch, for the Commissary ADR)
   alongside shipped-code citations `[VERIFIED: PROMOTION.md:264]`.
5. Update the "Next free ADR number" line to **0052** and append a dated note (matching the style of
   every prior multi-ADR-in-one-phase note in PROMOTION.md, e.g. the 2026-08-12 note for Phase 14's
   three ADRs) `[VERIFIED: PROMOTION.md:100-110 — worked precedent for exactly this shape]`.
6. Add three rows to `.planning/PROJECT.md`'s `## Key Decisions` table, each linking the new ADR file
   — the table links, never restates `[VERIFIED: PROMOTION.md:266, PROJECT.md:1310 comment]`.

### Pattern 2: The ADR is a NEW on-branch record, not a copy of the abandoned ADR-0010
**What:** VOCAB-02 requires the Commissary ADR be "reconstructed from" ADR-0010 and the port
commits — not a verbatim copy. CONTEXT.md is explicit: "Cite ADR-0010 by its abandoned-branch path;
do not copy it wholesale as a live ADR — the new ADR is the on-branch record."
**Source material verified this session (do not re-fetch — quoted below is authoritative):**

**ADR-0010's design** (`git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`, 2026-08-04, `Accepted`):
- Two responsibilities: `Quartermaster::verify_fits` (pre-flight GUARD, never trims,
  `ContextOverflow { measured_tokens, allotted_tokens, provider }`) and `Quartermaster::apportion`
  (bounded ALLOCATOR: fixed material + a `Convoy` of `ConvoyItem`s → an `Allotment` with retained
  items clamped/truncated-with-marker and every shed item recorded in `Allotment.shed`). "Nothing is
  dropped silently" — explicitly naming `rag_retrieval_service.rs:171`'s `truncate_to_token_budget`
  as the anti-pattern being rejected.
- `Quartermaster::new` refuses to invent a window: `UndeclaredContextWindow` unless the caller
  supplies `AllotmentConfig::fallback_context_tokens` explicitly.
- Honesty clause: neither claude-* nor deepseek-* has an offline exact tokenizer;
  `PESSIMISTIC_TOKENS_PER_1000_BYTES = 358` (measured 2.8-5.0 bytes/token range from debug session
  `deductive-32000-zero-output`); `TokenCounter::is_exact()` / `Allotment.exact_tally` make the
  estimate-vs-exact distinction type-level.
- **The three rejected candidate names (ADR-0010's own "Vocabulary" bullet), verbatim reasons:**
  `Muster` (collides with the framework's own troop-assembly vocabulary — the `paladin muster` CLI
  command, `Commands::Muster`); `provision()` (collides with `SandboxPort::provision`, a downstream
  container-lifecycle method); `ContextRation`/`Allocation` (collide with live downstream domain
  vocabulary — `rationale` is a judge/decision term, `allocation` is the downstream Prover's
  on-chain entitlement invariant `paid(x) <= allocation(x)`).

**The port commits** (`348f5910` "feat(paladin-llm): port v0.10.0-native prompt-budgeting
Commissary service", `35fd8390` "test(paladin-llm): port Commissary unit tests + wire module +
facade export"): the Quartermaster→Commissary rename rationale is stated in `348f5910`'s own body —
"Re-ports the removed Quartermaster/Convoy/apportion prompt-budgeting capability upstream as a
fresh v0.10.0-native service" — and its own avoided-name list, distinct from ADR-0010's: `Convoy`,
`apportion`, `Provisioner`, `ProvisioningPlan`, `Muster`/`muster` (all "verified-free medieval-
military names, 0/0 grep across both repos, per D-01/D-02" per the commit body — note this
supersedes ADR-0010's own now-outdated in-repo name `Quartermaster` itself, which the port commit
correctly avoids reintroducing). `is_exact_counter` was threaded as an explicit constructor
parameter because the new `TokenCounterPort::count` (v0.10.0) is infallible and carries no
exactness signal, unlike the old fallible `TokenCounter::is_exact()`.

**Combined rejected-name list the new ADR should carry** (union of both sources, deduplicated):
`Quartermaster` (retired, itself renamed away), `Convoy`, `apportion`, `Provisioner`,
`ProvisioningPlan`, `Muster`/`muster`, `provision()`, `ContextRation`, `Allocation`.

**Current shipped API** (`crates/paladin-llm/src/services/commissary.rs`, verified read this
session): `Commissary::new(provider, capabilities, counter, is_exact_counter, config)`,
`Commissary::from_port(llm, counter, is_exact_counter, config)`, `Commissary::verify_fits`,
`Commissary::dispense`, `Commissary::allotted_tokens`; types `CommissaryPlan`, `Consignment`,
`ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile` (fields: `dispensed`, `shed`,
`prompt_tokens`, `allotted_tokens`, `exact_tally`), `CommissaryError` (variants:
`UndeclaredContextWindow`, `ReservationExceedsWindow`, `FixedMaterialExceedsAllowance`,
`ContextOverflow`, `InvalidConfig`). Exported unconditionally (not feature-gated) from the facade at
`src/lib.rs:199-202`.

### Pattern 3: mdBook page content must mirror an existing test, never invent API
**What:** CONTEXT.md requires the Commissary usage sketch be "a real, compiling shape (a doc test or
an `ignore`-marked block that mirrors an existing unit test in `commissary.rs`)".
**Concrete source to mirror:** `commissary.rs`'s own `#[cfg(test)] mod tests` (verified read this
session, lines 600-1009) has 17 tests. The clearest end-to-end shape to mirror for a doc page is the
`commissary()` test helper (lines 631-640) plus `an_over_budget_consignment_sheds_the_lowest_priority_item_first`
(lines 664-686) — it exercises `Commissary::new` → `Consignment::push` (two items, different
priority) → `dispense` → reads `stockpile.shed` / `stockpile.dispensed`, which is exactly the
concept + model + usage-sketch shape VOCAB-03 asks for. Use `rust,ignore` (not a live doctest) since
`MockCounter`/`capabilities_with_window` are test-only helpers not in the public API — inventing a
public substitute would violate the "never invented API" rule; alternatively use `Commissary::from_port`
against `MockLlmAdapter` (feature `mock`), mirrored from `the_window_comes_from_the_ports_declared_capabilities`
(lines 985-1008), which uses only public API and could be a REAL doctest if the `mock` feature is
reachable from a doc-test context — the planner should verify feature-gating before committing to a
live vs. `ignore`-marked doctest; default to `ignore`-marked as the safe choice matching CONTEXT.md's
"or an ignore-marked block" option.

### Pattern 4: Three vocabulary lists must independently gain "Commissary" — none is generated
**What:** VOCAB-01 requires `PROJECT.md` AND `docs/src/architecture/domain-model.md` state the rule
and both list `Commissary`; CONTEXT.md additionally recommends (Claude's Discretion, "recommended,
one row") adding it to the `CLAUDE.md`/`copilot-instructions.md` table so all three agree.
**Verified current state of each list this session:**
- `.planning/PROJECT.md:1236-1239` (`## Constraints`, "Ubiquitous language" bullet): enumerates
  Paladin, Battalion, Formation, Phalanx, Campaign, Chain of Command, Conclave, Council, Grove,
  Maneuver, Commander, Garrison, Arsenal, Armament, Citadel, Herald, Armory, Sanctum, Sentinel,
  Quest. **No Commissary, no Treasurer.**
- `docs/src/architecture/domain-model.md` "Medieval Military Naming Convention" table (verified
  read this session, 18 rows: Paladin, Battalion, Formation, Phalanx, Campaign, Chain of Command,
  Conclave, Council, Grove, Maneuver, Commander, Garrison, Sanctum, Arsenal, Armament, Citadel,
  Herald, Quest). **No Commissary row.** This is also where the vocabulary-RULE prose (units/ports
  plain, roles medieval) should be added as a new subsection — the file currently states the naming
  convention as a flat mandate ("Use these terms...") with no plain-vs-medieval split articulated
  anywhere.
- `.github/copilot-instructions.md` "Naming Convention: Medieval Military Theme" table (14 rows:
  Paladin, Battalion, Formation, Phalanx, Campaign, Chain of Command, Commander, Garrison, Arsenal,
  Armament, Citadel, Herald, Armory, Quest — this is the project-instructions file loaded into every
  Claude session via `CLAUDE.md`'s `@.github/copilot-instructions.md` import). **No Commissary row,
  no Sanctum/Sentinel/Conclave/Council/Grove/Maneuver rows either** (this table is visibly stale
  relative to `domain-model.md`'s 18-row table — out of scope to reconcile fully, but adding
  Commissary here without reconciling the rest is consistent with CONTEXT.md's narrow instruction:
  "must not contradict the new rule", not "must be exhaustive").
- `CLAUDE.md` itself carries no separate table — it `@`-imports `copilot-instructions.md`, so editing
  the imported file's table is sufficient; do not create a duplicate table in `CLAUDE.md` proper.

Recommended vocabulary-rule wording (paraphrase, not prescriptive — Claude's Discretion per
CONTEXT.md): "Units and measures (`TokenUsage`, `max_tokens`, `max_context_tokens`, `TokenBudget`)
and technical port traits (`TokenCounterPort`, `LlmPort`, `EmbeddingPort`) keep plain industry
names. Domain roles, places and events get Medieval-Military names (Commissary, Treasurer, ...)."

### Anti-Patterns to Avoid
- **Copying ADR-0010 wholesale as the new ADR body.** CONTEXT.md is explicit this is wrong — cite
  it by its abandoned-branch path, reconstruct the substance.
- **Inventing a `Commissary` public API surface for the mdBook usage sketch** that does not exist in
  `commissary.rs` today (e.g. a convenience constructor, a builder pattern) — the service ships
  exactly the API enumerated in Pattern 2 above; no more, no less.
- **Editing `.planning/` phase history** (e.g. Phase 26 plans that record the Quartermaster→
  Commissary port) to remove "Quartermaster" — explicitly forbidden by the milestone overview §5.4
  and restated in CONTEXT.md/ROADMAP success criterion 5. The exit grep
  (`grep -rniE '\bQuartermaster\b' crates src`) does not even reach `.planning/` — scope is `crates`
  and `src` only.
- **Treating the `Treasurer` guardrail as this-repo enforcement.** The collision (`GarrisonTreasury`)
  is in the *downstream* Web3 Security Paladin repo, not in this tree — `grep -rn Treasurer crates
  src` returning empty in *this* repo is the exit condition; the guardrail text in the ADR is a
  cross-repo warning, not a local lint rule to build.
- **Registering a `MIGRATION.md` §9.2 row or a semver-checks allowlist entry for this phase's own
  edits.** Every touched surface (comments, rustdoc prose, ADRs, mdBook pages, planning-corpus
  tables) is non-compiling-surface or non-public-API prose. VOCAB-05's own text is explicit: "a
  rustdoc-only change, no `MIGRATION.md` §9.2 entry, no semver-checks allowlist change." This
  applies to every other edit in this phase by the same logic — none of them touches a public Rust
  API signature.

## Don't Hand-Roll

Not applicable in the conventional sense (no new runtime capability is being built), but two
process shortcuts to avoid:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Verifying the ADR heading shape | A new/simplified heading template | `0048-paladin-eval-composition-crate.md`'s exact heading order, verified in this file | CONTEXT.md names this file explicitly as the shape to copy; `adr-parser.cjs` depends on exact heading text for `## Code Locations`/`## Considered Options` bullet parsing |
| Checking mdBook link health | A custom grep for `.md` links | `mdbook build docs/` (linkcheck preprocessor, `warning-policy = "error"`) | The tool is already installed locally at the CI-pinned version; a hand-rolled link grep would miss anchor-level (`#section`) breakage that `mdbook-linkcheck` catches |

**Key insight:** this phase's only "build" risk is scope creep into code — every requirement is
satisfied by prose, an ADR, or a doc table. Any plan task proposing to touch a `.rs` file's
*signature* (not its doc comment) is out of scope per CONTEXT.md's phase boundary.

## Runtime State Inventory

Not applicable — this phase is not a rename/refactor/migration phase (Quartermaster is already
fully retired functionally; this phase only removes two remaining *prose* references). No stored
data, live service config, OS-registered state, secrets, or build artifacts carry the string
`Quartermaster` as a functional identifier anywhere in the tree — verified this session:
`grep -rniE '\bQuartermaster\b' crates src` returns exactly one hit, `src/lib.rs:195`, which is a
plain `//` comment (not a symbol, not a config key, not a secret name). The milestone overview's own
§3 anchor table confirms: "No functional Quartermaster symbol... No `struct/enum/impl/mod/use
Quartermaster` or `Quartermaster::` anywhere" `[VERIFIED: Milestone-13_Token-Economy.md:125]`.

## Common Pitfalls

### Pitfall 1: Treating `Treasurer` as needing any code-adjacent artifact
**What goes wrong:** A planner instinct to "reserve" a term in code (e.g. a marker trait, an empty
module, a `#[allow(dead_code)]` stub) to make the reservation "real."
**Why it happens:** Most GSD phases that "reserve" something also scaffold it.
**How to avoid:** CONTEXT.md and both PRDs are explicit: Treasurer is reserved *in writing only*,
0/0 in-tree, built in Milestone 14. Any code artifact for it in Phase 30 is out of scope and would
itself trip the exit grep `grep -rn Treasurer crates src` (which must stay empty).
**Warning signs:** A plan task that creates a new file, module, or feature flag mentioning
`Treasurer` in `crates/` or `src/`.

### Pitfall 2: `garrison.max_tokens`'s meaning is easy to misdescribe
**What goes wrong:** `docs/src/getting-started/configuration.md:248` labels `garrison.max_tokens`
"Context-window token budget" — this reads like it could be confused with the per-request
completion cap or the run-level `token_budget` cap.
**Why it happens:** The existing doc prose ("Context-window token budget") and the new table's
framing ("Garrison store cap") need to agree without contradicting each other; `GarrisonEntry` in
`domain-model.md` shows a `token_count: usize` field per stored entry, and `max_tokens` here caps
the Garrison's own retained-context total, not any single LLM call.
**How to avoid:** In the new table, name it precisely as CONTEXT.md's own success criterion 4
phrases it — "Garrison store cap" — and keep the existing inline comment at line 248 unchanged (it
already says "Context-window token budget", which does not contradict "Garrison store cap"; no edit
needed there beyond the new table).
**Warning signs:** A table row that says "context window" for the Garrison row — that phrase is
already used for the *provider's* declared window (`ProviderCapabilities::max_context_tokens`,
`Commissary`'s subject), a different concept entirely from the Garrison's stored-entry cap.

### Pitfall 3: The per-request completion cap has no single canonical config key
**What goes wrong:** Unlike the other three `max_tokens` meanings, the per-request completion cap is
NOT a single named YAML config key today — it flows through `LlmRequest.metadata["max_tokens"]` (a
`HashMap<String, String>` entry, read by `openai/adapter.rs:583-595` and used as a fallback-override
pattern) and, independently, through `AnthropicConfig.max_tokens` / the `ANTHROPIC_MAX_TOKENS` env
var (`anthropic/adapter.rs:53,80-94,206-257`, required by the Anthropic API itself).
**Why it happens:** OpenAI/DeepSeek treat `max_tokens` as an optional per-request override with no
adapter-level default; Anthropic requires `max_tokens` on every request and therefore has a real
config-level default.
**How to avoid:** Word the table's third row generically — "per-request completion cap" — and note
in prose (not as a config key column value) that it is set via `LlmRequest` metadata or, for
Anthropic, `llm.anthropic.max_tokens`/`ANTHROPIC_MAX_TOKENS`. Do not invent a single fictitious
unified key name (e.g. `llm.max_tokens`) that doesn't exist in `src/config/`.
**Warning signs:** A table row implying one global YAML key governs the per-request cap uniformly
across all three providers — it doesn't; this is provider-adapter-specific today.

### Pitfall 4: `copilot-instructions.md`'s table is already stale — don't over-scope the fix
**What goes wrong:** Noticing the copilot-instructions.md table is missing Sanctum, Sentinel,
Conclave, Council, Grove, Maneuver (in addition to Commissary) might tempt a full table
reconciliation.
**Why it happens:** Seeing a stale table next to the task of adding one row invites "fix it
properly while I'm here."
**How to avoid:** CONTEXT.md's Claude's Discretion note only asks that the table "must not
contradict the new rule" — reconciling the other six missing rows is out of this phase's scope
(VOCAB-01 names `Commissary` specifically; nothing in the requirements asks for a full table audit).
Add the one row; leave the rest.
**Warning signs:** A plan task titled anything like "reconcile ubiquitous-language tables."

## Code Examples

### Commissary construction + dispensing (mirrors `commissary.rs` test, for the mdBook usage sketch)
```rust,ignore
// Source: crates/paladin-llm/src/services/commissary.rs (test module, lines 631-686) —
// mirrored, not invented. Use `rust,ignore` since MockCounter/capabilities_with_window
// are test-only helpers; a real doctest would need a public counter (e.g. from
// paladin_memory::garrison::token_counter::HeuristicTokenCounter) and a real
// ProviderCapabilities from an LlmPort adapter, per Pattern 3 above.
use std::sync::Arc;
use paladin_llm::services::commissary::{Commissary, CommissaryPlan, Consignment, ConsignmentItem};
use paladin_ports::output::llm_port::ProviderCapabilities;

let capabilities = ProviderCapabilities {
    max_context_tokens: Some(100),
    ..Default::default()
};
let commissary = Commissary::new(
    "deepseek",
    capabilities,
    /* counter: Arc<dyn TokenCounterPort> */ counter,
    /* is_exact_counter */ false,
    CommissaryPlan::default(),
)?;

let mut consignment = Consignment::new();
consignment.push(ConsignmentItem { label: "high-priority".into(), body: "A".repeat(300), priority: 1 });
consignment.push(ConsignmentItem { label: "low-priority".into(),  body: "B".repeat(300), priority: 2 });

let stockpile = commissary.dispense("", &consignment)?;
// stockpile.shed[0].label == "low-priority"  (lower priority == shed first)
// stockpile.dispensed[0].label == "high-priority"
```

### `verify_fits` pre-flight guard (never trims)
```rust,ignore
// Source: crates/paladin-llm/src/services/commissary.rs:561-576
match commissary.verify_fits(&assembled_prompt) {
    Ok(measured_tokens) => { /* proceed to call the provider */ }
    Err(CommissaryError::ContextOverflow { measured_tokens, allotted_tokens, provider }) => {
        // fail loud — never silently truncate
    }
    Err(other) => { /* CommissaryError variants: UndeclaredContextWindow, ReservationExceedsWindow, InvalidConfig */ }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `Quartermaster`/`Convoy`/`apportion` (abandoned `feature/quartermaster-prompt-budgeting` branch, ADR-0010) | `Commissary`/`Consignment`/`dispense` (shipped, `paladin-llm`) | Port commits `348f5910`/`35fd8390` (undated in this session's fetch, precedes Phase 26 per milestone overview F4) | Same algorithm, renamed vocabulary; the old branch's ADR-0010 is now purely historical — cited, not live |
| No documentation home for Commissary | This phase adds an ADR + mdBook page | Phase 30 | Closes finding F4 partially (adoption/production-caller half is Phase 33 / COMM-01..03) |
| `ExecutionMetadata.cost_estimate` doc says nothing about who produces it | Rustdoc explicitly says "reserved for Treasurer (Milestone 14 / FUT-08); no in-tree producer yet" | Phase 30 (VOCAB-05) | Prevents a future reader assuming the field is live; sets expectation for Milestone 14 |

**Deprecated/outdated:**
- `Quartermaster`/`Convoy`/`apportion`/`Provisioner`/`ProvisioningPlan`/`Muster` (as budgeting
  vocabulary — `Muster` itself remains live elsewhere in the framework as the dynamic-fan-out
  feature, CF-03; only its *reuse* as a budgeting term is what's rejected) — superseded by
  `Commissary`/`Consignment`/`dispense`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The recommended vocabulary-rule wording ("Units/measures and technical port traits keep plain names; domain roles/places/events get Medieval-Military names") is a paraphrase for planner convenience, not the mandated final text — CONTEXT.md leaves exact wording to Claude's Discretion | Architecture Patterns, Pattern 4 | Low — CONTEXT.md explicitly grants wording discretion; any accurate paraphrase satisfies VOCAB-01 |
| A2 | Whether `Commissary::from_port` against `MockLlmAdapter` (feature `mock`) is reachable as a LIVE doctest from `docs/src/*.md` (mdBook code blocks are not compiled/tested by `mdbook build`, only by a separate `mdbook test` step which this project's CI does not appear to run) was not independently confirmed — recommend `rust,ignore` as the safe default per CONTEXT.md's stated fallback option | Pattern 3 | Low — CONTEXT.md explicitly permits either a doctest or an `ignore`-marked block; defaulting to `ignore` cannot fail a build |

**If this table is empty:** N/A — two low-risk clarifications above; no claim in this research is
unverified against the tree or an authoritative source.

## Open Questions

1. **Does `docs/src/architecture/domain-model.md`'s "Bounded Contexts" table (bottom of file) also
   need a Commissary/token-budgeting row?**
   - What we know: The table lists 7 bounded contexts (Agent execution, Memory, Orchestration, Tool
     integration, State persistence, Content ingestion, Storage) mapped to crates and an aggregate
     root. `Commissary` lives in `paladin-llm`, already listed under "Agent execution."
   - What's unclear: Whether VOCAB-01/03's requirement is satisfied by the Naming Convention table
     row alone, or whether a Bounded Contexts row is also expected.
   - Recommendation: The Naming Convention table row is sufficient — VOCAB-01's exact wording is
     "in both the ubiquitous-language list and the domain-model table," which the Naming Convention
     table satisfies; the Bounded Contexts table is a different, orthogonal classification (crate→
     aggregate-root) that Commissary (a *service*, not an aggregate root) doesn't naturally fit
     without forcing a new row shape. Leave it out.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `mdbook` | VOCAB-03 doc build verification | ✓ | 0.4.40 (matches CI pin exactly) | — |
| `mdbook-mermaid` | Optional Commissary flow diagram | ✓ | 0.13.0 (matches CI pin exactly) | — |
| `mdbook-linkcheck` | VOCAB-03 "link-check green" requirement | ✓ | 0.7.7 (matches CI pin exactly) | — |
| `git` access to `origin/feature/quartermaster-prompt-budgeting` | VOCAB-02 ADR provenance | ✓ | fetched and read this session via `git show` | — |
| `cargo doc` | VOCAB-05 rustdoc verification | ✓ (standard toolchain) | — | — |

**Missing dependencies with no fallback:** none.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None in the conventional sense — this is a docs-only phase. Verification is build/grep-based. |
| Config file | `docs/book.toml` (mdBook config, `warning-policy = "error"` on linkcheck) |
| Quick run command | `grep -rniE '\bQuartermaster\b' crates src` (VOCAB-06); `grep -rn Treasurer crates src` (VOCAB-04) — both must return empty/exit 1 |
| Full suite command | `mdbook build docs/` (from repo root: `cd docs && mdbook build` or `mdbook build docs/`) — exercises linkcheck across the whole book, ~1 minute per the docs.yml comment at line 20 |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VOCAB-01 | Vocabulary rule + Commissary in `PROJECT.md` + `domain-model.md` | manual/grep | `grep -n "Commissary" .planning/PROJECT.md docs/src/architecture/domain-model.md` (both must hit) | ✅ files exist |
| VOCAB-02 | ADR-0049 exists with required headings, rename rationale, rejected-name list | manual + grep | `ls .planning/decisions/0049-*.md`; `grep -c "^## " .planning/decisions/0049-*.md` (expect 7) | ❌ Wave — ADR not yet written |
| VOCAB-03 | mdBook Commissary page reachable from nav, linkcheck green | build | `mdbook build docs/` (exit 0) | ❌ Wave — page not yet written |
| VOCAB-04 | Treasurer reservation ADR-0050, 0/0 grep | manual + grep | `ls .planning/decisions/0050-*.md`; `grep -rn Treasurer crates src` (expect empty) | ❌ Wave — ADR not yet written |
| VOCAB-05 | `max_tokens` table (4 rows) + `cost_estimate` rustdoc | manual + `cargo doc` | `grep -c "max_tokens" docs/src/getting-started/configuration.md`; `cargo doc -p paladin-core --no-deps` (exit 0, no new warnings) | ❌ Wave — table/rustdoc not yet edited |
| VOCAB-06 | Quartermaster purge, `.planning/` untouched | grep | `grep -rniE '\bQuartermaster\b' crates src` (expect empty) | ✅ current state already known (one hit at `src/lib.rs:195`) |
| VOCAB-07 | Versioning ADR-0051 + `PROJECT.md` Key Decisions row | manual + grep | `ls .planning/decisions/0051-*.md`; `grep -n "0051" .planning/PROJECT.md` (expect a Key Decisions row) | ❌ Wave — ADR not yet written |

### Sampling Rate
- **Per task commit:** the relevant grep/`ls` check for that task's requirement (fast, <1s each)
- **Per wave merge:** `mdbook build docs/` (full linkcheck pass, ~1 min) + `cargo doc -p paladin-core
  --no-deps` (rustdoc-only crate, fast) + the full `grep -rniE '\bQuartermaster\b' crates src` and
  `grep -rn Treasurer crates src` exit-gate checks
- **Phase gate:** all seven requirement checks above green, plus `git diff --stat` confirming no
  `.rs` file outside `herald.rs`'s doc-comment lines and `src/lib.rs`'s comment line 195 was touched
  (a mechanical guard against scope creep into code)

### Wave 0 Gaps
None — no test *framework* gap exists because this phase produces no testable Rust behavior. The
"Wave 0" work is instead the grep/build commands above becoming runnable, which they already are
(all tools installed, all target files already exist to be edited).

## Security Domain

`security_enforcement` is not present in `.planning/config.json` (absent = enabled), so this section
is included per the protocol, scoped honestly to what actually applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No auth surface touched |
| V3 Session Management | No | No session surface touched |
| V4 Access Control | No | No access-control surface touched |
| V5 Input Validation | No | No new input-parsing code; `Commissary`'s existing `verify_fits`/`dispense` validation is unchanged by this phase (documentation only) |
| V6 Cryptography | No | No cryptographic code touched |

### Known Threat Patterns for this stack

No threat patterns apply — this phase writes no code, parses no untrusted input, and opens no new
attack surface. The one item worth naming explicitly for the record: the two Quartermaster prose
edits and the ADR text itself contain no secrets, credentials, or PII, and the milestone corpus
files being cited (`.project/Milestone_13-Token-Economy/`, `.project/Milestone_14-Treasurer/`) are
already committed, non-sensitive planning documents. The manual credential-handling review from
`security.instructions.md` has nothing to review in this phase's diff.

## Sources

### Primary (HIGH confidence — verified directly against this repo's tree this session)
- `crates/paladin-llm/src/services/commissary.rs` — full file read, public API, all 17 tests
- `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md` — full ADR-0010 text (168 lines)
- `git show -s --format=%B 348f5910` / `35fd8390` — full commit bodies
- `.planning/decisions/PROMOTION.md` — numbering scheme, next free number (0049), required headings, supersession mechanism, promotion procedure
- `.planning/decisions/0048-paladin-eval-composition-crate.md` — heading-shape precedent
- `crates/paladin-core/src/platform/container/herald.rs` — exact `cost_estimate` line numbers (387, 471, 521, 551, 607)
- `crates/paladin-herald/src/json_herald.rs:208`, `markdown_herald.rs:343` — confirmed `cost_estimate` has no producer anywhere in the tree (`grep -rn cost_estimate crates src`)
- `src/lib.rs:195` — the sole `Quartermaster` prose hit (`grep -rniE '\bQuartermaster\b' crates src`)
- `.project/project-management/paladin-project-plan-final.md:1112` — `SirQuartermaster` example
- `docs/src/architecture/domain-model.md`, `docs/src/SUMMARY.md`, `docs/src/getting-started/configuration.md`, `docs/book.toml`, `.github/workflows/docs.yml` — full/partial reads
- `.github/copilot-instructions.md` (via CLAUDE.md's import, reproduced in system context) — Naming Convention table
- `.planning/PROJECT.md` — Current Milestone section (lines 281-367), Key Decisions table format, Constraints "Ubiquitous language" bullet (1236-1239)
- `mdbook --version` / `mdbook-mermaid --version` / `mdbook-linkcheck --version` — local tool availability, exact version match against CI pins
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md`, `Epic_1/prd-vocabulary-and-docs-foundation.md` — full reads
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md`, `Epic_1/prd-treasurer-spend-governance.md` — full reads
- `.project/v0.10.0/00-program-overview.md:44` — X-03 exact text
- `crates/paladin-ports/src/output/llm_port.rs`, `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`, `src/config/agent_runtime.rs`, `src/application/services/paladin/middleware/limits.rs` — the four `max_tokens` meanings' exact code locations

### Secondary (MEDIUM confidence)
- None — every claim in this research was verified directly against the tree this session; nothing
  relies on WebSearch or unverified training knowledge.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; existing toolchain verified installed at exact CI-pinned versions
- Architecture: HIGH — every file/line cited was read directly this session
- Pitfalls: HIGH — derived from direct code inspection (e.g. the per-request `max_tokens` key ambiguity across providers), not speculation

**Research date:** 2026-09-14
**Valid until:** Stable — this is a docs-only phase against an already-shipped, frozen code surface (v0.10.0 pre-tag, feature-frozen except Phases 31-33's own later breaking changes). Re-verify line numbers (`herald.rs` ~387/471/521/551/607, `src/lib.rs:195`) immediately before editing, per this research's own citations, since Phases 31-33 executing after Phase 30 could shift `herald.rs` line numbers (ACCT-01/02 touch `TokenUsage`-adjacent code) — but Phase 30 itself lands first per the dependency order, so this is a non-issue for Phase 30's own execution, only a note for anyone re-reading this file after Phase 31 has landed.
