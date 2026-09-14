# Phase 30: Token-Economy Vocabulary & Commissary Anchoring - Context

**Gathered:** 2026-09-14
**Status:** Ready for planning
**Source:** PRD Express Path (`.project/Milestone_13-Token-Economy/Epic_1/prd-vocabulary-and-docs-foundation.md`), amended by the roadmap-time decisions recorded in `ROADMAP.md` (Phase 30 entry + 2026-09-14 extension footer) and `REQUIREMENTS.md` (VOCAB-01…07)

<domain>
## Phase Boundary

Docs-only, non-breaking. Freeze the token-economy vocabulary in writing, give the shipped
`Commissary` service (`crates/paladin-llm/src/services/commissary.rs`, facade-exported, no in-tree
caller, no docs page, design ADR only on an abandoned branch — finding F4) a documentation home,
reserve the `Treasurer` term with a one-page ADR and its downstream guardrail, document the four
meanings of `max_tokens` and the reserved `cost_estimate` field, delete the last two orphan
`Quartermaster` references, and record the clean-break versioning decision that Phases 31-33
depend on as an ADR.

Output is documentation, three ADRs, one mdBook page, comment/rustdoc edits, and two prose edits.
**No code behaviour changes. No type, function, field or config key is renamed.** This phase can
land alone and first; Phases 31-33 build on it.

</domain>

<decisions>
## Implementation Decisions

### Vocabulary rule (D-1 → VOCAB-01)
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

### Commissary anchoring (D-2 → VOCAB-02, VOCAB-03)
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

### Treasurer reservation (D-3 → VOCAB-04)
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

### `max_tokens` disambiguation and `cost_estimate` reservation (D-8, D-9 → VOCAB-05)
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

### Quartermaster purge (VOCAB-06)
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

### Versioning decision ADR (VOCAB-07 — roadmap-time addition, not in the PRD)
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

### Locked by the milestone overview §0 (do not revisit)
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone corpus (source of truth for this phase)
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` — locked
  decisions (§0), findings F1-F8 / decisions D-1…D-9 (§4), verified anchors (§3), cross-repo
  constraints incl. the `Treasurer` guardrail (§5), out-of-scope (§7)
- `.project/Milestone_13-Token-Economy/Epic_1/prd-vocabulary-and-docs-foundation.md` — R1-R8,
  tests/verification, exit criteria
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` and
  `Epic_1/prd-treasurer-spend-governance.md` — what the reservation ADR promises the Treasurer
  will own (do not build any of it)

### Planning record
- `.planning/ROADMAP.md` — Phase 30 entry (goal, success criteria 1-6) and the 2026-09-14
  extension footer (the three scope-time amendments)
- `.planning/REQUIREMENTS.md` — VOCAB-01…07 and the 2026-09-14 extension record
- `.planning/PROJECT.md` — Current Milestone section (extension paragraph, X-03 supersession
  paragraph), Key Decisions table (row format to extend)
- `.planning/decisions/PROMOTION.md` — ADR numbering scheme, required headings, supersession rule
- `.planning/decisions/0048-paladin-eval-composition-crate.md` — the latest ADR; copy its
  heading shape (`# ADR-NNNN: title`, `## Status`, `**Date:**`, `## Context`, …)
- `.project/v0.10.0/00-program-overview.md` §3 X-03 (line 44) and X-10 — the rule VOCAB-07
  supersedes, and the semver-hygiene rule that keeps applying

### Commissary provenance (for the Commissary ADR)
- `crates/paladin-llm/src/services/commissary.rs` — module docs (design, `is_exact_counter`
  rationale, the silent-truncation anti-pattern it names), public types, unit tests
- `crates/paladin-llm/src/services/mod.rs` and `src/lib.rs` (facade re-export block around
  line 195) — export surface and the orphan provenance comment
- `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`
  — the abandoned-branch ADR-0010 (accepted 2026-08-04): declared-window budgeting rationale,
  refusal semantics; the original service lived at
  `crates/paladin-llm/src/services/quartermaster.rs` on that branch
- `git show -s --format=%B 348f5910` / `35fd8390` — the port commits; `348f5910`'s body lists
  the names avoided at port time (`Quartermaster`, `Convoy`, `apportion`, `Provisioner`,
  `ProvisioningPlan`, `Muster`/`muster` — "verified-free medieval-military names, 0/0 grep across
  both repos, per D-02") and the abandoned ADR-0010 §Decision "Vocabulary" bullet (line ~84)
  records three earlier candidates "rejected on research, not taste" — together these are the
  rejected-name list the Commissary ADR must carry
- `.planning/phases/26-agent-runtime-enhancements/26-11-PLAN.md` (D-13 lines 23-28) — the
  Phase 26 context-window decisions: infallible `TokenCounterPort`, the RAG `len()/4` left
  in place under X-03, the legacy `garrison::TokenCounter` untouched — history, do not edit

### Docs surface being edited
- `docs/book.toml` — `[output.linkcheck] warning-policy = "error"`, mermaid preprocessor
- `docs/src/SUMMARY.md` — nav (architecture entries at lines ~36-40)
- `docs/src/architecture/domain-model.md` — domain-model table to extend
- `docs/src/getting-started/configuration.md` — where the `max_tokens` table goes
- `.github/workflows/docs.yml` — pinned mdbook / mermaid / linkcheck versions
- `CLAUDE.md` + `.github/copilot-instructions.md` — developer-facing ubiquitous-language table

### Code touched (rustdoc / comment only)
- `crates/paladin-core/src/platform/container/herald.rs` — `ExecutionMetadata.cost_estimate`
  rustdoc (field, accessor, builder, example)
- `src/lib.rs:195` — provenance comment
- `.project/project-management/paladin-project-plan-final.md` — `SirQuartermaster` example

</canonical_refs>

<specifics>
## Specific Ideas

- Exit grep (PRD §5): `grep -rniE '\bQuartermaster\b' crates src` → empty. Also confirm
  `grep -rn Treasurer crates src` → empty at ADR-authoring time (the "0/0 verified" claim).
- The three ADRs are separate documents: Commissary design+rename record, Treasurer
  reservation, token-economy versioning (X-03 supersession). Each is an `Accepted` ADR dated
  2026-09-14.
- The mdBook page should show the `Consignment` → `Commissary::dispense` → `Stockpile` /
  `ShedItem` flow (a mermaid diagram is welcome — the mermaid preprocessor is installed).
- The `max_tokens` table has exactly four rows plus the `allowance` sentence; keep it in the
  configuration page, not a new page.
- Rustdoc edits must keep `cargo doc` / intra-doc links clean and `cargo clippy -- -D warnings`
  green; `make clean-code` before every commit (project working agreement).
- Commit granularity: ADRs, mdBook page, docs tables, rustdoc/comment edits can be separate
  conventional commits (`docs(30): …`); the pre-commit hook runs workspace clippy (~70-140 s
  warm) — commit with `git commit` directly rather than the GSD commit helper (known timeout).

</specifics>

<deferred>
## Deferred Ideas

- Building the Treasurer (pricing, `cost_estimate` producer, allowances, pacing) — Milestone 14,
  `.project/Milestone_14-Treasurer/`; FUT-08 / FUT-09 stay v2.
- Any code-behaviour change, any rename of shipped types, `TokenUsage` carrier changes
  (Phase 31), `TokenCounterPort::is_exact` / `Commissary::new` signature / legacy counter
  removal / shared window resolver (Phase 32), RAG adoption of `Commissary` (Phase 33).
- Rewriting `.planning/` phase history that mentions Quartermaster — forbidden (overview §5.4).

</deferred>

---

*Phase: 30-token-economy-vocabulary-commissary-anchoring*
*Context gathered: 2026-09-14 via PRD Express Path*
