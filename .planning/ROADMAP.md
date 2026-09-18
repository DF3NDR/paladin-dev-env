# Roadmap: Paladin

## Overview

**Paladin already works.** It ships at v0.7.0 with a Cargo workspace of ten library crates plus a
`doc-examples` crate and the root `paladin-ai` facade, 22 runnable examples, an HTTP API with
OpenAPI and SSE streaming, a `paladin-server` binary, a 112 MB multi-arch Docker image and
reference Kubernetes manifests. (**Amended by Phase 4, dated 2026-08-03, citing
`04-release-measurement.md`**: the "22 runnable examples" figure traces to a Milestone 1 Epic 10
validation report ("22/22 examples compiling") and has since gone stale — the shipped tree carries
**47** `.rs` files under `examples/`, of which four are declared `[[example]]` targets gating on
non-default features (`vision`, `content-processing`, `web-server`); no crate under `crates/` ships
its own `examples/` directory. The shipped tree outranks an ingested count under this project's
precedence order. Going forward the gate REL-05 and ROADMAP criterion 5 express is "every example
target builds", not a count, so this figure cannot go stale the same way again.)

**This planning corpus is a historical record of twelve shipped milestones plus a verified-defect
and deferred-work forward scope. It is not a greenfield plan.** Across the 263 documents in
`.project/`, 7,511 of 8,053 task items are checked (93%) — and five runs of direct code
verification found the shipped tree *ahead of* even that figure in most places. **This roadmap does
not build the framework. It closes out milestones that already shipped, fixes what verification
proved broken, and builds the one epic-set nobody ever started.**

The sixteen phases fall into four kinds of work, and it is worth naming them before the detail:

| Kind | Phases | What it is |
|---|---|---|
| **Record** | 1, 5, 7, 10, 13 | Make `.planning/` describe the code as it actually is, so nobody re-plans shipped work or applies a superseded PRD literally |
| **Verified defect** | 8, 12 | Fix the things direct verification proved broken — a permanently red CI job, missing annotations, disabled doctests, leaked dependencies, a duplicated audit job |
| **Decision** | 9, 11, 14 | Answer the questions the corpus left open, including the ones with a correctness or security consequence attached |
| **Genuinely unbuilt** | 2, 3, 4, 6, 15, 16 | The residual functional gaps, the quality gates, and the two deferred registers whose work was never started |

### The milestone arc this roadmap closes out

**M1-M3 built capability.** Paladin, Garrison, Arsenal, the four base Battalion patterns, Herald,
Citadel, Commander and the Armory CLI (M1); then Sanctum vector memory and RAG, Sentinel vision,
autonomous agents, Conclave, Council, Grove and the Maneuver Flow DSL (M2); then the completion
pass over all of it (M3).

**M4-M8 dismantled and rebuilt the structure that capability lived in**, at considerable cost and
with almost no feature work: feature-flag expansion and port hardening (M4), the monolith becoming
a Cargo workspace (M5), four layer relocations (M6), four more crate extractions and the first
crates.io publish (M7), and a facade cleanup that a dated reconciliation then audited against the
tree and took further than its own plan allowed (M8).

**M9-M12 finished, hardened, documented and exposed it.** M9 completed the half of the platform
M4-M8 had left alone — a real `execute_workflow()`, a workflow repository with crash recovery,
scheduler/queue/event validation, the bidirectional content-agent bridge and user/admin RBAC. M10
made it releasable: pre-commit, cargo-audit + cargo-deny + OSV-Scanner, a CycloneDX SBOM,
cargo-release with dependency-ordered publishing, and — after an incident — main-only tag
enforcement. M11 documented it into an mdbook with 227 broken links repaired and linkcheck as an
error. M12 exposed it over HTTP, **and it exists because M11's documentation epic wrote down a
capability gap instead of papering over it.**

### What the phases do, in order

**Milestone 1 close-out (Phases 1-4)** is short and specific. Make the planning record match the
shipped code and give each of the six contested type/gate definitions one recorded answer
(Phase 1). Close the residual functional gaps verification exposes and apply those definitions in
code (Phase 2). Make the quality numbers real rather than aspirational (Phase 3). Make the release
coherent — one version, one edition, a defensible advisory posture, reviewed docs, the whole gate
suite green (Phase 4).

**Milestone 2-3 close-out (Phases 5-6)** is shorter still, and that is the finding rather than an
omission. Sanctum, RAG, Sentinel vision, autonomous planning and handoffs, Conclave, Council,
Grove, the Maneuver Flow DSL, the enhanced CLI, Herald consolidation, the Paladin registry and the
scheduler port **all ship in the v0.7.0 tree.** What is missing is the record (Phase 5). Exactly one
defect in run-2 scope is verified open, and it closes alongside whatever Phase 5 exposes (Phase 6).

**Milestone 4-6 close-out (Phases 7-8)** covers the three milestones that restructured what M1-M3
built. All of it shipped, and unusually for this corpus it was verified directly against
`Cargo.toml` contents and type definitions rather than inferred. Phase 7 records what shipped and
answers the variant pairs; **Phase 8 is the first phase whose scope is entirely verified defects.**

**Milestone 7-8 close-out (Phases 9-11)** is the first block where the *record* is in better shape
than the gates. The 2026-06-04 reconciliation is the most reliable document in the corpus — every
verifiable claim in it matches the tree, including a `println!` residue count exact to the
occurrence. So Phase 9 fixes the gates rather than the record and **carries the only dated item in
the corpus, a RustSec acceptance expiring 2026-09-30**. Phase 10 writes down what M7-M8 delivered.
Phase 11 disposes of the deferred registers.

**Milestone 9-12 + Deferred-QA close-out (Phases 12-16)** is where the last of the forward work
lives, and it splits cleanly. Phase 12 deletes eighteen lines of CI that falsify a completed
milestone's own success metric, and gives thirteen advisory suppressions an owner and a date.
Phase 13 records what four milestones delivered and answers two seams M12 left as defaults.
Phase 14 closes the gap between what the project's interfaces *advertise* and what they *do* — an
API documented as JWT and implemented as opaque tokens, a Kubernetes Deployment against an
in-process token store, and an LLM capability flag that over-reports. Phase 15 builds the quality
gates Deferred-QA Epic 25 specified and nobody started, then closes the coverage register those
gates measure. Phase 16 finishes Milestone 11's documentation currency — **the only open checkbox
count in all 542 that survives verification** — and decides the fate of an architecture document
frozen at 311 lines that two milestones made invisible.

## Milestones

| Milestone | Phases | Status | Source |
|---|---|---|---|
| **Milestone 1 close-out** | 1-4 | ✅ **Shipped v0.7.1 (2026-08-04)** — [archive](milestones/v0.7.1-ROADMAP.md) | Ingest run 1 — `.project/Milestone_1-MVP` (36 docs) |
| **Milestone 2-3 close-out** | 5-6 | ✅ **Shipped v0.8.0 (2026-08-24)** — [archive](milestones/v0.8.0-ROADMAP.md) | Ingest run 2 — `.project/Milestone_2-Missing_features` + `.project/Milestone_3-Completion` (45 docs) |
| **Milestone 4-6 close-out** | 7-8 | ✅ **Shipped v0.8.0 (2026-08-24)** — [archive](milestones/v0.8.0-ROADMAP.md) | Ingest run 3 — `.project/Milestone_4-Refactor-Crates-Features` + `.project/Milestone_5-Workspace-Decomposition` + `.project/Milestone_6-Architectural-Refinements` (32 docs) |
| **Milestone 7-8 close-out** | 9-11 | ✅ **Shipped v0.8.0 (2026-08-24)** — [archive](milestones/v0.8.0-ROADMAP.md) | Ingest run 4 — `.project/Milestone_7-Production-Hardening` + `.project/Milestone_8-Facade-Cleanup-Shim-Resolution` (40 docs) |
| **Milestone 9-12 + Deferred-QA close-out** | 12-16 | ✅ **Shipped v0.8.0 (2026-08-24)** — [archive](milestones/v0.8.0-ROADMAP.md) | Ingest run 5 (FINAL) — `.project/Milestone_9-Classic-Orchestrator-Completion` + `.project/Milestone_10-CI-Hardening-Release-Automation` + `.project/Milestone_11-Documentation-Overhaul-Publish` + `.project/Milestone_12-Web-API` + `.project/Deferred-QA-CICD-Completion` + `.project/project-management` (46 docs) |
| **Provider Expansion** | 17 | ✅ **Shipped v0.8.0 (2026-08-24)** — [archive](milestones/v0.8.0-ROADMAP.md) | Forward work — not ingest-derived. Added 2026-08-15 per *Roadmap Extension Protocol* item 1. |
| **Security Tooling** | 18-21 | ✅ **Shipped v0.9.0 (2026-09-01)** — [archive](milestones/v0.9.0-ROADMAP.md) | Forward work — not ingest-derived. Added 2026-08-24 per *Roadmap Extension Protocol* item 1, closing the Rust-SAST gap the v0.8.0 milestone audit left as its one genuinely open item; extended 2026-08-25 with Phases 19-21 (publish credential, publish operations, release artifacts). |
| **Durable Agent Execution Runtime** | 22-37 | 🔄 **In progress** — Phases 22-29 complete 2026-09-10 (`0.10.0` bumped on the feature branch, tag not yet cut); Phases 30-33 added 2026-09-14, complete 2026-09-16; Phases 34-37 added 2026-09-17 (documentation currency + the crate release) | Forward work — not ingest-derived. Added 2026-09-01, sourced from the user-authored design corpus in `.project/v0.10.0/` (program overview `00`, epic PRDs `01`-`07`, traceability matrix `08`) rather than the historical `.project/Milestone_*` ingest. **Extended 2026-09-14** with Phases 30-33 (Token Economy — Commissary anchoring, lossless accounting, primitive unification, Commissary adoption), sourced from `.project/Milestone_13-Token-Economy/` (overview + Epics 1-4). Milestone 14 (Treasurer, `.project/Milestone_14-Treasurer/`) is reserved, not roadmapped. **Extended 2026-09-17** with Phases 34-37 (Release Readiness — documentation currency audit, mdBook currency, rustdoc zero-warning bar & examples currency, the crate release) — operator-instructed pre-tag work, not corpus-sourced. |

**The ingest is complete.** All 263 documents in `.project/` are covered — 199 classified across
five runs and 64 `tasks-*.md` measured deterministically by `intel/task-completion-state.md`. There
is no run 6. The *Roadmap Extension Protocol* at the end of this file still governs any future
addition, but nothing is pending.

Milestone numbering follows the **directory / task-list numbering**. Four source milestones number
themselves differently — the M4-M6 overviews use refactoring tiers ("Milestone 1/2/3"), the M3
release notes assign Epics 19-23 to four M2 features, and the M7 overview titles itself
"Milestone 4" — and none of those labels is used as a key anywhere in this file (VERIFY-03,
ARCH-02, HARD-04). **The protocol predicted a fifth instance in run 5; run 5 found none, and
ORCH-05 records the prediction closed.**

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>✅ <strong>Milestone 1 close-out (Phases 1-4)</strong> — SHIPPED v0.7.1 2026-08-04 · 38 plans, 25/25 requirements</summary>

- [x] **Phase 1: Ground Truth & Decision Records** - Verify the planning record against shipped v0.7.0 code and record one answer per competing variant pair (completed 2026-07-31)
- [x] **Phase 2: Functional Gap Closure** - Finish the residual Milestone-1 functionality and apply the recorded definitions in code (completed 2026-08-01)
- [x] **Phase 3: Verification Depth** - Make coverage, error-path testing and performance baselines real and measured (completed 2026-08-02)
- [x] **Phase 4: Release Coherence** - One version, one edition, defensible dependencies, reviewed docs, green gate suite (completed 2026-08-03)

Full detail: [`milestones/v0.7.1-ROADMAP.md`](milestones/v0.7.1-ROADMAP.md) ·
Audit: [`milestones/v0.7.1-MILESTONE-AUDIT.md`](milestones/v0.7.1-MILESTONE-AUDIT.md) ·
Phase artifacts: `milestones/v0.7.1-phases/`

</details>

<details>
<summary>✅ <strong>v0.8.0 — Milestone 2-12 close-out &amp; Provider Expansion (Phases 5-17)</strong> — SHIPPED 2026-08-24 · 149 plans, 65/65 requirements</summary>

**Milestone 2-3 close-out**

- [x] **Phase 5: Milestone 2-3 Ground Truth** - Record what Epics 11-24 actually shipped, verify the three unverified blocks, and fix the epic-numbering defect at its source (completed 2026-08-05)
- [x] **Phase 6: Verified Gap Closure** - Close the one verified defect plus whatever Phase 5 proves genuinely outstanding (completed 2026-08-05)

**Milestone 4-6 close-out**

- [x] **Phase 7: Workspace Ground Truth & Recorded Answers** - Record what the refactor milestones actually shipped, correct the five positions the code contradicts, and answer the four variant pairs and two policy questions (completed 2026-08-06)
- [x] **Phase 8: Verified Defect Closure** - Fix the five defects verification proved open: the broken API-surface CI job, missing deprecations, disabled port doctests, leaked CLI dependencies, and duplicate `TokenUsage` (completed 2026-08-07)

**Milestone 7-8 close-out**

- [x] **Phase 9: Release & Security Gate Integrity** - Reconcile the four divergent RustSec exception sets before the 2026-09-30 expiry, settle the licence posture, and close the three small release-gate defects (completed 2026-08-08)
- [x] **Phase 10: Milestone 7-8 Ground Truth & Recorded Account** - Record what production hardening and facade cleanup actually delivered, make the 2026-06-04 reconciliation authoritative, and answer the three architecture questions the documents left ambiguous (completed 2026-08-08)
- [x] **Phase 11: Facade Residue & Deferred Register Disposition** - Give each of the five deferred items and both deliberately removed features a decision, and triage the Milestone 9 candidate list (completed 2026-08-09)

**Milestone 9-12 + Deferred-QA close-out**

- [x] **Phase 12: Supply-Chain Gate Integrity** - Delete the duplicate audit job that falsifies a completed milestone's success metric, and give every advisory suppression an owner and a date (completed 2026-08-10)
- [x] **Phase 13: Milestone 9-12 Ground Truth & Recorded Account** - Record what the orchestrator, release-automation, documentation and Web API milestones delivered, and turn two recorded defaults into decisions (completed 2026-08-10)
- [x] **Phase 14: API Contract Truthfulness** - Make every capability the project advertises through an interface one it actually has — the token mechanism, the multi-replica store, and the LLM capability flag (completed 2026-08-12)
- [x] **Phase 15: Coverage & CI Quality Gates** - Build the quality gates Deferred-QA Epic 25 specified and nobody started, then close the coverage register those gates measure (completed 2026-08-13)
- [x] **Phase 16: Documentation Currency & the Architecture Gap** - Settle Milestone 11's fourteen content-currency files by content, and decide whether the 311-line architecture document is archive or deliverable (completed 2026-08-24)

**Provider Expansion** — first forward work beyond the ingest (added 2026-08-15)

- [x] **Phase 17: Additional LLM Provider Adapters** - Decide which additional providers qualify against recorded criteria, then ship each survivor as a feature-gated adapter meeting the full `LlmPort` contract (completed 2026-08-23)

Full detail: [`milestones/v0.8.0-ROADMAP.md`](milestones/v0.8.0-ROADMAP.md) ·
Audit: [`milestones/v0.8.0-MILESTONE-AUDIT.md`](milestones/v0.8.0-MILESTONE-AUDIT.md) ·
Requirements: [`milestones/v0.8.0-REQUIREMENTS.md`](milestones/v0.8.0-REQUIREMENTS.md) ·
Phase artifacts: `milestones/v0.8.0-phases/`

</details>

<details>
<summary>✅ <strong>v0.9.0 Security Tooling (Phases 18-21)</strong> — SHIPPED 2026-09-01 · 25 plans, 20/20 requirements</summary>

- [x] **Phase 18: Rust SAST — Evaluate and Adopt CodeQL** - Prove a Rust-capable SAST actually analyses this tree before adopting it, then wire it as a non-blocking scan and only afterwards as a required check (completed 2026-08-25 — verdict: disqualified at CodeQL 2.26.3, retained advisory-only)
- [x] **Phase 19: crates.io Trusted Publishing — Replace the Long-Lived Registry Token** - Exchange the standing `CARGO_REGISTRY_TOKEN` secret for OIDC-issued ephemeral publish tokens, prove the new path works before revoking the old credential, and record the per-crate trust configuration the eleven-crate workspace needs (completed 2026-08-28)
- [x] **Phase 20: Release Pipeline Recovery — Idempotent Re-Runs and a Pre-Publish Gate** - Make a re-run on the same tag the supported way to finish a half-published release, refuse to publish until tag, manifest versions, changelogs and the tagged commit's CI conclusion agree, and write the stuck-halfway runbook including a yank policy (completed 2026-08-30)
- [x] **Phase 21: Release Artifacts — Curated Release Notes and Attached Distributables** - Build the release body from the curated `CHANGELOG.md` section instead of a commit log, and make the attached distributables real: binaries that actually compile under the features their targets require, an image bound to the release by digest, and verifiable checksums (completed 2026-09-01)

Full detail: [`milestones/v0.9.0-ROADMAP.md`](milestones/v0.9.0-ROADMAP.md) ·
Audit: [`milestones/v0.9.0-MILESTONE-AUDIT.md`](milestones/v0.9.0-MILESTONE-AUDIT.md) ·
Requirements: [`milestones/v0.9.0-REQUIREMENTS.md`](milestones/v0.9.0-REQUIREMENTS.md) ·
Phase artifacts: `milestones/v0.9.0-phases/`

</details>

**Durable Agent Execution Runtime (Phases 22-33)** — in progress (added 2026-09-01; extended 2026-09-14 with Phases 30-33)

- [x] **Phase 22: Battlefield State & Superstep Engine** - Typed shared state, cyclic superstep execution, and automatic per-superstep checkpointing that resumes with zero re-execution after a crash (completed 2026-09-02)
- [x] **Phase 22.1: Engine readiness defect and MSRV follow-up (INSERTED)** - Fix the BUG-03 cycle-bootstrap starvation and BUG-04 resume-frontier defects, complete the graph fingerprint, raise the MSRV floor to a measured 1.88, and seal G-22-1 on whole-run CI evidence (completed 2026-09-03)
- [x] **Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs** - Directive-based routing, Muster dynamic fan-out, nested Battalion subgraphs, LLM-evaluated routing, and the BUG-01 fail-closed fix (completed 2026-09-04)
- [x] **Phase 24: Pause/Resume, History & Graceful Shutdown** - Indefinite Parley pauses, typed resume validation, an inspectable/forkable Chronicle, graceful shutdown, and Thread endpoints over HTTP (completed 2026-09-05)
- [x] **Phase 25: Node-Level Fault Tolerance** - Typed error transience, per-node Aegis retry, wall/idle timeouts, typed compensation handlers, provider fallback, and node result caching (completed 2026-09-06)
- [x] **Phase 26: Agent Runtime Enhancements** - Execution middleware chain, context-window management, cross-session Vault memory, structured output, provider conformance close-out, and a one-line reasoning agent (completed 2026-09-07)
- [x] **Phase 27: Platform API** - Durable background runs on a worker pool, Parley/streaming integration, versioned assistants, and API-managed schedules/webhooks (completed 2026-09-08)
- [x] **Phase 28: Observability & Tooling** - Machine-consumable trace stream, OTel/log/SSE consumers, graph/run visualization, and the paladin-eval regression harness (completed 2026-09-09)
- [x] **Phase 29: Program Gates & Release** - Complete MIGRATION.md, proven backward compatibility, the program acceptance audit, and a releasable v0.10.0 (completed 2026-09-10)

**Token Economy — Commissary anchoring & lossless accounting** (added 2026-09-14, still v0.10.0 — the `0.10.0` tag is not yet cut; Phase 33 re-seals the Phase 29 release gates)

- [x] **Phase 30: Token-Economy Vocabulary & Commissary Anchoring** - Record the units-plain / roles-medieval vocabulary rule, anchor `Commissary` with an ADR and an mdBook page, reserve `Treasurer` with its downstream guardrail, document the four `max_tokens` meanings, purge the orphan `Quartermaster` references, and record the clean-break versioning decision as an ADR (docs only) (completed 2026-09-14)
- [x] **Phase 31: Lossless Token Accounting** - Carry the full `TokenUsage` prompt/completion split (plus optional cache/reasoning fields) from the LLM port to `RunFinished` and a herald, remove the `from_total` zeroing from the battalion path, and prove streaming usage parity per adapter (keystone; breaking) (completed 2026-09-15)
- [x] **Phase 32: Unified Token Primitives** - One counting contract (`TokenCounterPort::is_exact`, `Commissary::new` drops `is_exact_counter`, legacy `TokenCounter`/`TokenCounterFactory` retired) and one shared context-window resolver with a strict mode consumed by both `HistoryTrimmer` and `Commissary` (breaking) (completed 2026-09-16)
- [x] **Phase 33: Commissary In-Tree Adoption** - Route RAG truncation through `Commissary::dispense` with shed records and a truncation marker, closing the last silent-truncation path with an integration-tested production caller, then re-seal the Phase 29 release gates on the final commit (completed 2026-09-16)

**Release Readiness — documentation currency & the crate release** (added 2026-09-17, still v0.10.0 — the `0.10.0` tag is not yet cut; Phase 37 cuts it)

- [x] **Phase 34: Documentation Currency Audit** - Audit the mdBook, the rustdoc corpus and the `examples/` / `doc-examples` programs against everything Phases 22-33 changed (and anything v0.9.0 left unwritten), producing one classified gap inventory that scopes Phases 35-36 (read-only; no docs change) (completed 2026-09-17)
- [x] **Phase 35: mdBook Currency** - Close every mdBook gap the Phase 34 inventory records so the book describes the v0.10.0 tree — new pages where a capability shipped without one, corrected pages where the API or vocabulary changed, and `mdbook build` + linkcheck green (completed 2026-09-17)
- [x] **Phase 36: Rustdoc Zero-Warning Bar & Examples Currency** - Take `cargo doc --workspace --no-deps` from 73 carried warnings to zero so CI's "Check documentation" step is green, resolve the 14 `--all-features` intra-doc links, and bring every `examples/` and `doc-examples` program current with the Phase 22-33 API (completed 2026-09-18)
- [ ] **Phase 37: v0.10.0 Crate Release** - Re-seal the Phase 29 release gates on the post-documentation final commit, merge to `main`, cut the `v0.10.0` tag through `release.yml`, and confirm every publishable crate is on crates.io at `0.10.0`

## Phase Details

*Phases 1-4 are archived in [`milestones/v0.7.1-ROADMAP.md`](milestones/v0.7.1-ROADMAP.md).
Phases 5-17 — every phase of the v0.8.0 milestone, with their full goals, success criteria,
amendment banners and per-plan checklists — are archived in
[`milestones/v0.8.0-ROADMAP.md`](milestones/v0.8.0-ROADMAP.md), together with
[`v0.8.0-REQUIREMENTS.md`](milestones/v0.8.0-REQUIREMENTS.md) and
[`v0.8.0-MILESTONE-AUDIT.md`](milestones/v0.8.0-MILESTONE-AUDIT.md). Phases 18-21 — every phase
of the v0.9.0 Security Tooling milestone — are archived in
[`milestones/v0.9.0-ROADMAP.md`](milestones/v0.9.0-ROADMAP.md), together with
[`v0.9.0-REQUIREMENTS.md`](milestones/v0.9.0-REQUIREMENTS.md) and
[`v0.9.0-MILESTONE-AUDIT.md`](milestones/v0.9.0-MILESTONE-AUDIT.md). Only phases in the current
and future milestones are detailed below, which is what keeps this file a constant size per
milestone. Phases 22-33 — the current milestone, v0.10.0 "Durable Agent Execution
Runtime" — are detailed in full below.*

### Phase 22: Battlefield State & Superstep Engine

**Goal**: A Rust developer can declare typed shared state exchanged through per-field dispatch rules, and run cyclic multi-agent graphs in supersteps that checkpoint automatically and resume with zero re-execution after a crash.
**Depends on**: Nothing (first phase of v0.10.0; builds on the v0.9.0 baseline)
**Requirements**: ENG-01, ENG-02, ENG-03, ENG-04, ENG-05, ENG-06, ENG-07, ENG-08
**Success Criteria** (what must be TRUE):

  1. A developer can declare a `BattlefieldSchema` and nodes exchange typed `StateDelta`s through per-field dispatch rules (`LastWrite`, `Append`, `MergeObject`, `Sum`, `Custom`), with unknown-field and missing-required schema violations surfacing as hard, structured `BattlefieldError`s, in `paladin-core` with no new core dependencies (ENG-01)
  2. The `WarEngine` executes cyclic graphs, self-loops included, in bounded supersteps with deterministic frontier and merge order — byte-identical Battlefields over 20+ randomized-scheduling iterations — and join/defer semantics that never deadlock on a not-firing branch (ENG-02)
  3. Exactly one Waypoint is persisted automatically after every superstep, addressed by `(thread_id, waypoint_id)` with parent lineage and a stable graph fingerprint, and a Waypoint write failure fails the run under the default `Strict` durability (ENG-03)
  4. Program scenario E2E-1 passes: an engine killed after superstep 3 is reconstructed fresh from the same backend and `thread_id`, resumes with zero re-execution of already-completed nodes, and reaches a final Battlefield identical to an uninterrupted control run, with exactly one Waypoint per completed superstep (ENG-04)
  5. Three `WaypointPort` backends (InMemory, SQLite, Postgres) all pass one shared contract suite; legacy `from_formation`/`from_phalanx`/`from_campaign` constructors reproduce today's data flow with golden output-equivalence tests; `MIGRATION.md` exists at the repository root with the §9 skeleton and pre-populated register entries; and the `cargo semver-checks` and MSRV CI jobs run green on every PR (ENG-05, ENG-06, ENG-07, ENG-08)

**Plans**: 17 plans (10 waves) — 11 original, plus 6 gap-closure plans from the UAT
**Wave 1**

- [x] 22-01-PLAN.md — Tracer: one typed node checkpointed end-to-end and resumed with zero re-execution (wave 1, blocking D-04 fingerprint decision)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 22-02-PLAN.md — Battlefield: typed accessors, schema enforcement, five-rule deterministic multi-writer merge (wave 2)
- [x] 22-03-PLAN.md — WaypointPort contract, ThreadId validation, shared generic contract suite on InMemory (wave 2)
- [x] 22-04-PLAN.md — Program scaffolding: MIGRATION.md §9, semver CI job, MSRV 1.85 CI job (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 22-05-PLAN.md — Engine core: WarGraph validation permitting cycles, superstep loop, bounded iteration (wave 3)
- [x] 22-06-PLAN.md — SQL Waypoint backends: SQLite, Postgres, migrations, retention config (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 22-07-PLAN.md — Engine frontier: join/defer, custom dispatch registry, determinism + stress test (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 22-08-PLAN.md — Paladin nodes, InputMapping, full resume, program scenario E2E-1 (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 22-09-PLAN.md — Engine seams: TraceSink, NodeInterceptor chain, CancellationToken to Halted (wave 6)
- [x] 22-10-PLAN.md — ENG-NFR benchmarks: Waypoint save overhead and engine memory per superstep (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 22-11-PLAN.md — Legacy bridges, golden output-equivalence tests, coverage close-out (wave 7)

**Gap closure** *(from `22-UAT.md`: G-22-1 major, G-22-2 blocker, G-22-3 major)*

**Wave 8** *(gap-closure wave 1; blocked on Wave 7 completion)*

- [x] 22-12-PLAN.md — G-22-1: postgres-integration CI job so the Postgres Tier 2 contract suite executes somewhere, with reachability, skip and empty-selection assertions (wave 8)
- [x] 22-13-PLAN.md — G-22-2: WaypointPort delete-one primitive and prune_thread keep-set contract, transactional on both SQL backends (wave 8)
- [x] 22-15-PLAN.md — G-22-3 / BUG-02: eligible-set reachability validation on WarGraph::validate with an explicit dynamic-target marker, test-first (wave 8)

**Wave 9** *(gap-closure wave 2)*

- [x] 22-14-PLAN.md — G-22-2: prune rebuilt on the keep-set primitive, protected set defined once in the application layer, fault-injection and resume acceptance test (wave 9)
- [x] 22-16-PLAN.md — G-22-3: fixture audit per acceptance 2a, plus the readiness defect it surfaces recorded with a runnable reproduction (wave 9)

**Wave 10** *(gap-closure wave 3)*

- [x] 22-17-PLAN.md — Blocking checkpoint: confirm the CI run actually exercised the Postgres suite, and settle the readiness defect's disposition (wave 10)

### Phase 22.1: Engine readiness defect and MSRV follow-up (INSERTED)

**Goal:** Close the three items Phase 22's gap-closure checkpoint (22-17) could not settle in-repo: (1) fix the frontier readiness defect found by the 22-16 audit — a node that is both self-looping and fed by an upstream edge can never take its first turn yet the run reports Completed (repro: `#[ignore]`d test `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`, `cargo test -p paladin-battalion --lib engine::superstep -- --ignored`); (2) decide and enact the MSRV position — the workspace declares rust-version 1.85 but the =2.1.0-pinned rmcp's `transport-child-process` feature requires process-wrap >=9.x (rustc 1.86/1.87 minimum), so either the MSRV raises (MIGRATION.md §9.3 / X-11.1 D-07 register) or the rmcp pin moves; (3) confirm a fully green `postgres-integration` CI run after the 22-17 fixes and record it as G-22-1's closing evidence; (4) close 22-REVIEW.md CR-01 — `WarGraph::fingerprint()` omits `defer_flags`/`dynamic_targets` from its hashed bytes, so resume's fingerprint-mismatch check cannot detect a defer-flag change (fix the hash + regression test, or narrow the 'structurally impossible' doc claim in engine/mod.rs). **Added 2026-09-03 (developer decision at the 22.1-05 checkpoint):** (5) fix BUG-04 — `WarEngine::resume` rebuilds the Frontier from scratch (`Frontier::new`), losing pre-crash edge resolutions, so a fired edge into a not-yet-ready join node is lost on resume and the resumed run diverges from the control run; persist the frontier on the Waypoint, fix test-first, register as BUG-04 / ENG-FR-12a, and re-capture the CI evidence on the final head.
**Requirements**: ENG-02, ENG-04, ENG-05
**Depends on:** Phase 22
**Plans:** 7/7 plans complete

Plans:
**Wave 1**

- [x] 22.1-01-PLAN.md — BUG-03 cycle-bootstrap starvation fix (tracer, test-first) + BUG-03/ENG-FR-06a registration
- [x] 22.1-04-PLAN.md — MSRV floor to measured 1.88, lockfile restore, resolver 3, MSRV figure reconciliation

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 22.1-02-PLAN.md — Truthful-outcome guards: validate-time unschedulable-shape check, run-end starvation check, determinism coverage

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 22.1-03-PLAN.md — CR-01 fingerprint coverage + golden/difference tests + resume-doc reword + fixture comment sweep

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 22.1-05-PLAN.md — G-22-1 closing CI evidence (whole-run success) + UAT pointer

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 22.1-06-PLAN.md — BUG-04 resume frontier loss: RED reproduction, `FrontierSnapshot` persisted on the Waypoint, three-backend contract cases, BUG-04 / ENG-FR-12a registration

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 22.1-07-PLAN.md — CI evidence re-capture on the final head after BUG-04 (dated section appended to 22.1-CI-EVIDENCE.md)

### Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs

**Goal**: Nodes steer their own routing at runtime, dynamically fan out into map-reduce workers, nest Battalions as subgraphs, and optionally route by LLM evaluation — with the BUG-01 custom-edge-condition defect fixed fail-closed.
**Depends on**: Phase 22
**Requirements**: CF-01, CF-02, CF-03, CF-04, CF-05
**Success Criteria** (what must be TRUE):

  1. BUG-01 is fixed fail-closed: an unregistered `EdgeCondition::Custom(name)` fails graph validation with `BattalionError::InvalidGraph`, naming every unregistered condition, before any node executes — on both `CampaignExecutionService` and the `WarEngine` — with the fix's failing-then-passing test visible in history (CF-01)
  2. A node's returned `Directive` (`NextStep::{Edges, Goto, End, Muster, Parley}`) steers execution with validated `Goto` targets and documented, tested End-over-Goto precedence, via a configurable `DirectiveParser` that defaults to backward-compatible `PlainOutput` (CF-02)
  3. A planner node's Directive musters a runtime-determined number of worker tasks in one superstep with payload isolation, deterministic `task_key`-ordered aggregation, duplicate-key rejection, a `max_muster_tasks` limit, and mid-muster resume that re-runs only unfinished tasks (CF-03)
  4. A Battalion can embed a child WarGraph via `NodeSpec::Battalion` with `StateMap` input/output mapping and private child fields, namespaced checkpoint inheritance with resume-mid-child, and recursive embedding rejected at validation (CF-04)
  5. An `LlmDecision` edge evaluator and Commander `StrategySelection::Semantic` are available and off by default, falling back to Heuristic on any LLM error with the fallback recorded, and existing Commander tests pass unmodified (CF-05)

**Plans**: 12 plans (9 waves)

Plans:
**Wave 1**

- [x] 23-01-PLAN.md — BUG-01 fail-closed on both paths (registered `EdgeConditionEvaluator`, RED-then-GREEN) + M-B-01 worked example (CF-01)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 23-02-PLAN.md — `Directive`/`NextStep`/`MusterTask` in core, `StateNode::run` return change, Goto/End/Parley arms (CF-02)
- [x] 23-03-PLAN.md — `LlmDecisionEvaluator` (one call per decision per superstep) + Commander `StrategySelection::Semantic` (CF-05)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 23-04-PLAN.md — per-node `DirectiveParser` for Paladin nodes: `PlainOutput` default, `StructuredDirective` envelope, `on_parse_error` (CF-02)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 23-05-PLAN.md — Muster fan-out: worker templates, one-superstep dispatch, `task_key` ordering, validation-before-dispatch, `muster.` namespace (CF-03)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 23-06-PLAN.md — mid-muster progress Waypoints, resume running only unfinished tasks, contract-suite coverage, ENG-FR-11 note (CF-03)
- [x] 23-07-PLAN.md — `EngineConfig` at `src/config/engine.rs` with `APP_ENGINE_*` overrides; closes MIGRATION §9.5 (CF-03)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 23-08-PLAN.md — `NodeSpec::Battalion` + `StateMap` subgraph composition, engine inheritance, recursion rejection (CF-04)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 23-09-PLAN.md — injective child `ThreadId`, `checkpoint_ns`, resume-mid-child, Formation-inside-Campaign test (CF-04)
- [x] 23-10-PLAN.md — graph fingerprint `v2` → `v3` with the six new hashed properties and a re-pinned golden (CF-02/03/04)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 23-11-PLAN.md — E2E-3 muster/defer/order integration test + 50-task multi-thread stress test (CF-03)

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 23-12-PLAN.md — mdBook control-flow page, §9.2 register closeout, CHANGELOG, program-gate evidence (CF-01…CF-05)

### Phase 24: Pause/Resume, History & Graceful Shutdown

**Goal**: A workflow can pause indefinitely for human input without holding compute, resume from a different process, expose an inspectable and forkable Chronicle, shut down without losing in-flight work, and be driven over HTTP.
**Depends on**: Phase 22, Phase 23
**Requirements**: HITL-01, HITL-02, HITL-03, HITL-04, HITL-05
**Success Criteria** (what must be TRUE):

  1. A node (or a first-class `Gate` node with Battlefield templating) raising a `ParleyRequest` suspends the run, persists an `AwaitingInput` Waypoint carrying all of the superstep's parleys, releases every resource, and is resumable from a different process sharing the same backend (HITL-01)
  2. Program scenario E2E-2 passes: `resume_with(graph, thread, responses)` validates typed responses per kind (Approval/Choice/FreeText/StateEdit) with typed errors that leave the thread suspended, honors `expires_at`, and correctly routes both branches of an approval gate across a process drop/recreate (HITL-02)
  3. History/inspect over `WaypointPort` supports `replay` and `fork`-with-edit, creating a new chain with `fork_of` lineage while the original chain stays byte-identical, with branch-aware latest resolution (HITL-03)
  4. Graceful shutdown finishes the in-flight superstep within `shutdown_grace` (default 30s), records over-grace nodes `Skipped` and re-lists them in the vanguard, `resume` continues a `Halted` thread, and SIGTERM/SIGINT are wired to all in-flight runs with `k8s/` manifests and docs updated and a documented disable switch (HITL-04)
  5. `GET /threads/{id}/state`, `POST /threads/{id}/resume` (with 409/400/404 semantics), and `GET /threads/{id}/history` (paginated) are reachable over HTTP following existing utoipa + error-envelope conventions, with `openapi.json` regenerated (HITL-05)

**Plans**: 14 plans (12 executed + 2 gap closure)

Plans:
**Wave 1**

- [x] 24-01-PLAN.md — Parley value types and the suspension/resume spine (tracer)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 24-02-PLAN.md — `Gate` node, graph validation, edge routing, fingerprint `v4`

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 24-03-PLAN.md — Directive parley envelope and the `parley.` InputMapping namespace

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 24-04-PLAN.md — `resume_with` validation matrix, partial answers, expiry

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 24-05-PLAN.md — E2E-2, multi-parley, cross-process and 10-thread stress tests
- [x] 24-06-PLAN.md — `fork_of` lineage, `child_on_branch`, three-backend contract cases

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 24-07-PLAN.md — `replay`/`fork`, `ChronicleService`, immutability and subgraph-fork

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 24-08-PLAN.md — Shutdown grace race, `ShutdownCoordinator`, `EngineConfig` fields

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 24-09-PLAN.md — Process wiring, k8s manifests, deployment docs, M-B-02 example
- [x] 24-10-PLAN.md — `ParleyPort`, facade adapter, `GraphRegistry`, `WaypointStoreConfig`

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 24-11-PLAN.md — Thread routes, DTOs, `openapi.json`, server composition

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 24-12-PLAN.md — mdBook page, MIGRATION/CHANGELOG/traceability, gate evidence

**Wave 11** *(gap closure — 24-VERIFICATION.md, blocked on Wave 10 completion)*

- [x] 24-13-PLAN.md — Doc drift after CR-01: MIGRATION §9.6 `403`, CHANGELOG, mdBook posture callout

**Wave 12** *(gap closure — blocked on Wave 11 completion)*

- [x] 24-14-PLAN.md — Blocking human diff read of the CR-02 mid-Muster shutdown-abort fix

### Phase 25: Node-Level Fault Tolerance

**Goal**: Individual nodes retry with provable backoff, distinguish stalled from slow work via nested timeouts, compensate typed errors instead of failing the run, fail over across LLM providers, and cache expensive deterministic results.
**Depends on**: Phase 22, Phase 23 (FT-04's E2E-3 depends on CF-03/Muster)
**Requirements**: FT-01, FT-02, FT-03, FT-04, FT-05, FT-06
**Success Criteria** (what must be TRUE):

  1. `transience()` on `PaladinError` and `LlmError` is table-driven per variant, provider adapters carry status-carrying error variants with no string parsing, and `BattalionError::Node(NodeError)` carries a structured `NodeError` through engine execution — every touched pre-existing public enum handled per X-10 and registered in `MIGRATION.md` §9.2 (FT-01)
  2. Per-node Aegis retry follows an exact backoff sequence under a paused clock with asserted jitter bounds, gates on the transience predicate (Permanent → 1 attempt), discards failed-attempt deltas while keeping `AttemptRecord` history, and retries per-task inside a Muster (FT-02)
  3. A wall-clock `run_timeout` and a progress-aware `idle_timeout` (stream chunks, trace events, `ctx.heartbeat()`) are distinguished and nested with engine/Battalion bounds so the tightest fires and the error names which (FT-03)
  4. Program scenario E2E-3 passes together with CF-03: a `Route`/`Absorb`/registered-`Custom` typed error handler compensates a transiently-failing Muster worker without failing the run, unregistered `Custom` names fail closed, and handler loops are bounded by `max_node_visits` (FT-04)
  5. `FallbackLlmAdapter` fails over across a provider chain on Transient/Unknown errors only (short-circuiting on Permanent) without silently switching providers mid-stream, and a `CachePolicy`-keyed node hits its `NodeCachePort` cache with `cache_hit: true` and no re-execution while failures are never cached (FT-05, FT-06)

**Plans**: 14 plans

Plans:
**Wave 1**

- [x] 25-01-PLAN.md — Tracer: core Transience/NodeError/Aegis value types, WarGraph aegis sidecar, superstep retry loop, backoff and predicate tests

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 25-02-PLAN.md — X-10 error taxonomy gate: transience() on both enums, LlmFailure/ProviderError/AllProvidersFailed/BattalionError::Node, three enums non-exhaustive, §9.2 rows
- [x] 25-03-PLAN.md — Aegis validation: EngineRegistries, fail-closed predicate/handler registries, node-kind matrix, fingerprint v4 → v5
- [x] 25-04-PLAN.md — Node cache port, CachedDelta, InMemory + Redis adapters under one contract suite, redis-cache feature, NodeCacheConfig

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 25-05-PLAN.md — One shared map_http_status helper (redact-then-bound) applied to all nine LLM provider adapters
- [x] 25-06-PLAN.md — LlmError → PaladinError::LlmFailure conversion helper and the eight erasure sites
- [x] 25-07-PLAN.md — Retry expansion: AttemptRecord history, per-attempt trace fields, structured failure path, per-task Muster retry

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 25-08-PLAN.md — FallbackLlmAdapter chain with streaming first-chunk rule, FallbackHop trace, PaladinResult.served_by

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 25-09-PLAN.md — Timeouts: HeartbeatHandle, defaulted PaladinPort::execute_observed, run/idle/EngineRun nesting, RunTimeoutExceeded

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 25-10-PLAN.md — Typed error handlers: Route/Absorb/Custom validation and dispatch, max_node_visits loop bound

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 25-11-PLAN.md — Handler composition: Muster delta-only rule, Parley from a handler, compensation-chain and loop-bound E2E

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 25-12-PLAN.md — E2E-3 seam replaced by real per-task retry, X-05 stress, kill-during-backoff, run-timeout E2E
- [x] 25-13-PLAN.md — Cache engine integration: key composition, hit/miss path, FieldSpec cache marker, correctness guardrails

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 25-14-PLAN.md — Fault-tolerance guide, MIGRATION §9.1-§9.7 close-out, traceability anchors, semver/MSRV/security/coverage gate evidence

### Phase 26: Agent Runtime Enhancements

**Goal**: `PaladinExecutionService` gains a middleware pipeline, context-window management, confined cross-session memory, first-class structured output, verified provider conformance, and a one-line tool-loop agent preset.
**Depends on**: Phase 22 (mostly standalone; parallelizable with Phases 24/25)
**Requirements**: RT-01, RT-02, RT-03, RT-04, RT-05, RT-06, RT-07
**Success Criteria** (what must be TRUE):

  1. An ordered `ExecutionMiddleware` chain (before/after model, around tool) applies onion ordering and short-circuit semantics under per-run state isolation, and the same chain applies when a Paladin runs as an engine node (RT-01)
  2. Built-in middleware ships config-structured per X-09: `ModelCallLimit`/`TokenBudget` (new `StopReason` variants), `ToolCallLimit` denying without failing the run, `Guardrail` prompt/response screens, and retry/fallback middleware that delegates to the FT-05 implementation without duplicating logic (RT-02)
  3. A `TokenCounterPort`, a stable never-splits-a-message `HistoryTrimmer`, and a compounding `SummarizationMiddleware` keep long conversations within the context window, degrading to trimming on summarizer failure and never failing the run (RT-03)
  4. A `VaultPort` (InMemory/SQLite/semantic) confines `vault_get`/`vault_put` Armaments to a host-granted namespace subtree (rejecting traversal), and `execute_structured<T>` returns schema-validated output through a bounded, typed repair loop that preserves raw output on exhaustion (RT-04, RT-05)
  5. The shipped v0.8.0 OpenAI-compatible/Gemini/Ollama paths pass a shared conformance suite with FT-01-correct 429/5xx transience mapping, and `reasoning_agent(llm, tools, opts)` runs as a ≤15-line doc-tested one-liner with tool failures fed back to the model by default (RT-06, RT-07)

**Plans**: 21 plans

Plans:
**Wave 1**

- [x] 26-01-PLAN.md — Tracer: the `ExecutionMiddleware` chain end-to-end (RT-01)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 26-02-PLAN.md — `AgentRuntimeConfig`: twelve X-09 sub-structs, inert by default (RT-02)
- [x] 26-03-PLAN.md — `LlmRequest` builder + `ResponseFormat` + 37-site migration (RT-05)
- [x] 26-04-PLAN.md — Vault core types, `VaultPort`, `InMemoryVault`, contract suite (RT-04)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 26-05-PLAN.md — `StopReason` X-10 treatment + the three limit middlewares (RT-02)
- [x] 26-06-PLAN.md — Native `response_format` in OpenAI / compat / Gemini / DeepSeek (RT-05)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 26-07-PLAN.md — `GarrisonEntry.is_summary` + one embedded migrator + `002` (RT-03)
- [x] 26-08-PLAN.md — `Guardrail` middleware + `PaladinError::GuardrailTripped` (RT-02)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 26-09-PLAN.md — `SqliteVault` + `003` + `SemanticVault` under the contract suite (RT-04)
- [x] 26-10-PLAN.md — Retry/fallback port-shaping middleware + `RetryPredicate::admits` (RT-02)
- [x] 26-12-PLAN.md — Structured core/ports machinery + the `extract_json` lift (RT-05)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 26-11-PLAN.md — `TokenCounterPort`, its adapters, and `HistoryTrimmer` (RT-03)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 26-13-PLAN.md — `ConfinedVault`, `RunScope`, `execute_scoped`, engine vault wiring (RT-04)
- [x] 26-14-PLAN.md — Shared conformance suite, measurement first, Ollama recipe (RT-06)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 26-15-PLAN.md — `SummarizationMiddleware` + `VaultRecallMiddleware` (RT-03, RT-04)

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 26-16-PLAN.md — `InProcessArsenal`, composite, `VaultTools`, confinement attack test (RT-04)

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 26-17-PLAN.md — `StructuredExecutorPort` impl + `StructuredExecutorExt` (RT-05)

**Wave 11** *(blocked on Wave 10 completion)*

- [x] 26-18-PLAN.md — Engine `output_schema`, schema registry, fingerprint `v6` (RT-05)

**Wave 12** *(blocked on Wave 11 completion)*

- [x] 26-19-PLAN.md — Tool-call protocol, tool-error policy, corrected M-B-03 (RT-07)

**Wave 13** *(blocked on Wave 12 completion)*

- [x] 26-20-PLAN.md — `reasoning_agent` preset, `build_chain`, the ≤15-line example (RT-07, RT-02)

**Wave 14** *(blocked on Wave 13 completion)*

- [x] 26-21-PLAN.md — Guide, `MIGRATION.md` sweep, api-surface regen, gate evidence (all)

### Phase 27: Platform API

**Goal**: Runs execute durably in the background on a worker pool, integrate with Parley pauses and live streaming, and are managed through versioned assistants, cron schedules and webhooks — all reachable over a production-shaped HTTP API.
**Depends on**: Phase 22, Phase 24
**Requirements**: PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `POST /runs` returns 202 within 250ms p99 (enqueue only), a `RunRepositoryPort` persists every status transition, and the status machine is monotonic with typed illegal-transition errors (PLAT-01)
  2. A worker pool executes runs via `RunQueuePort` (InMemory/Redis) with lease heartbeats, at-least-once redelivery that resumes a kill-mid-run thread rather than restarting it, cross-instance cancellation observed at superstep boundaries, and a `409 ThreadBusy` invariant holding under 10 concurrent submits (PLAT-02)
  3. `AwaitingInput` releases the worker, `POST /threads/{id}/resume` re-enqueues under the same `run_id`, and `GET /runs/{id}/stream` bridges live TraceSink events to SSE with a documented polling-backed degraded mode and 15s heartbeats (PLAT-03)
  4. Assistants are append-only immutable versions (no PUT, ever) with `latest` frozen at submit time, and `WarGraphDoc` compiles through a registry-resolving `compile()` with a restart-stable fingerprint round-trip (PLAT-04)
  5. Cron schedules survive restart without duplicate or missed-then-double firing; HMAC-signed webhook delivery retries bounded on 5xx/timeout with an SSRF guard rejecting non-http(s)/loopback/link-local/private/metadata targets; and every new endpoint carries existing auth, rate limiting, scopes and pagination, with `openapi.json` regenerated and Python/TypeScript clients generated and smoke-tested in CI (PLAT-05, PLAT-06)

**Plans:** 26/26 plans complete

Plans:

**Wave 1**

- [x] 27-01-PLAN.md — Tracer: core `Run`/status machine (D-01 checkpoint), ports, InMemory adapters, submission + worker, `POST /runs` → `Completed` end-to-end (PLAT-01, PLAT-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 27-02-PLAN.md — `runs` migrations, run-repository contract suite, SQLite + Postgres adapters, partial unique index (PLAT-01, PLAT-02)
- [x] 27-03-PLAN.md — `RunQueuePort` contract suite, Redis ZSET+Lua lease adapter, `redis-queue` CI job, generalised Postgres job (PLAT-02)
- [x] 27-04-PLAN.md — Worker pool hardening: heartbeat, resume-not-restart dispatch, drain, kill-mid-run twin of acceptance 2 (PLAT-02, PLAT-03)
- [x] 27-05-PLAN.md — `WarGraphDoc` + `compile()`, schemars golden schema, fixture corpus, two-process fingerprint proof (PLAT-04)
- [x] 27-06-PLAN.md — Seven X-09 config structs, all off/today by default (PLAT-01…05)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 27-07-PLAN.md — `CancellationProbe` engine seam, debounced DB probe, persisted-flag-first cancel, cross-instance test (PLAT-02)
- [x] 27-08-PLAN.md — Resume re-enqueues the same `run_id`; `ResumeAccepted`/`ResumeAcceptedResponse.run_id`; §9.2/§9.6 (PLAT-03)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 27-09-PLAN.md — Assistant storage: core types (D-28/D-29 checkpoint), update-less port, migrations, three adapters, freeze-at-submit (PLAT-04)
- [x] 27-10-PLAN.md — Run streaming: `RunEventBus`, TraceSink adapter, degraded mode, SSE route with 15 s keep-alive (PLAT-03)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 27-11-PLAN.md — Schedules storage + `ScheduleService`: croner/chrono-tz, conditional tick claim, restart/race proofs under a paused clock (PLAT-05)
- [x] 27-12-PLAN.md — Assistant service: compile-is-validation, stored resolver, `DocGraphRegistry`, assistant routes with synthetic code entries (PLAT-04)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 27-13-PLAN.md — Webhooks: delivery table/adapters, SSRF guard (write + send), HMAC over exact bytes, no-redirect client, bounded-retry drain (PLAT-05)
- [x] 27-14-PLAN.md — `ScheduleAdminPort` + `/v1/schedules` routes (PLAT-05, PLAT-06)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 27-15-PLAN.md — HTTP surface completion: runs list/cancel/deliveries, threads list/get/fork/delete, scopes, pagination, 429 proof, ten-concurrent-submits (PLAT-06, PLAT-02)
- [x] 27-16-PLAN.md — mdBook platform-api page, queue/worker + k8s worker-replica example, parley page links (PLAT-05, PLAT-06)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 27-17-PLAN.md — `paladin-server` wiring from config, fail-closed feature gates, services registered with the coordinator, §9.5 (all)

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 27-18-PLAN.md — Acceptance-1 E2E test, `sdk-clients` CI job, phase-wide OpenAPI review, §9.6, CI evidence checkpoint (all)

**Gap closure — Wave 1** *(from 27-VERIFICATION.md's 5 CI-evidenced gaps + 27-REVIEW.md CR-01/WR-01…04; parallel, disjoint files)*

- [x] 27-19-PLAN.md — Redis claim/nack scripts increment `attempt` only on a reclaim; Tier-1 marker guards (PLAT-02)
- [x] 27-20-PLAN.md — Postgres run-timestamp microsecond precision contract; contract fixtures at storage resolution (PLAT-01)
- [x] 27-21-PLAN.md — Hermetic `sdk-clients` smoke: loopback LLM stub, committed lockfile, exact `completed` assertion (PLAT-06)
- [x] 27-22-PLAN.md — Webhook hardening: bounded response-body read (CR-01), no send on signing-key load failure (WR-01) (PLAT-05)
- [x] 27-23-PLAN.md — Heartbeat zero-lease guard (WR-04); `Agent`-kind delivery carve-out and unscoped-read model documented, tested, ledgered (WR-02, WR-03) (PLAT-02, PLAT-03, PLAT-05, PLAT-06)

**Gap closure — Wave 2** *(blocked on Wave 1: the public-API baseline is taken after every code change)*

- [x] 27-24-PLAN.md — Toolchain-order-independent API-surface extraction + regenerated baseline; `e2e-platform-api` CI job (PLAT-06)

**Gap closure — Wave 2.5** *(added 2026-09-08 from CI run 34238527001 at the Wave-2 SHA: the one remaining red cause once 27-19 made later `run_all` clauses reachable)*

- [x] 27-26-PLAN.md — `contract_tests::run_all` provisions a fresh queue per clause (factory), exercised on both backends; unblocks `redis-queue`, `coverage`, `integration-tests` (PLAT-02)

**Gap closure — Wave 3** *(blocked on Wave 2)*

- [x] 27-25-PLAN.md — CI evidence checkpoint: live-run proof for every closed gap, recorded in 27-CI-EVIDENCE.md (all)

### Phase 28: Observability & Tooling

**Goal**: Every run emits a machine-consumable trace that reaches real consumers, graphs and runs are visualizable, and agent behavior is regression-testable.
**Depends on**: Phase 22 (trace seam), Phase 27 (WarGraphDoc)
**Requirements**: OBS-01, OBS-02, OBS-03, OBS-04
**Success Criteria** (what must be TRUE):

  1. The authoritative `TraceEvent` enum carries a per-run monotonic `seq` with a gapless-or-counted-drops guarantee, and a `TraceSinkPort` whose slow or panicking implementations cannot stall or fail a run fans out via `CompositeSink` (OBS-01)
  2. Traces reach a default-on structured-log sink, an `otel`-gated OpenTelemetry exporter with span-per-attempt trees verified against a collector stub, and the SSE bridge for `GET /runs/{id}/stream` as a TraceSink adapter (OBS-02)
  3. Golden-tested `WarGraphDoc → Mermaid/DOT` exporters and an execution-overlay export let a human answer "which branch fired and why did node X run 3 times" via `paladin-cli graph export`/`run export` and a minimal auth-gated `dev-ui` inspector page (OBS-03)
  4. The new `paladin-eval` crate runs scripted mock-LLM scenario files through a `cargo test`-integrable runner macro and `paladin-cli eval run --repeat`/`--bless`, with the three program E2E fixtures dogfooded as eval scenarios (OBS-04)

**Plans:** 17/17 plans complete

Plans:
**Wave 1**

- [x] 28-01-PLAN.md — Core trace types, `TraceRecord` envelope, `TraceEmitter`, `CompositeSink`, dispatcher stamping, panic isolation, drop accounting, ordering + X-05 stress tests (OBS-01)
- [x] 28-02-PLAN.md — `TraceConfig`/`OtelConfig` config structs, `web_server.dev_ui.mermaid_url`, both YAML files (OBS-01, OBS-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 28-03-PLAN.md — Engine producers: `EdgeEvaluated`, `ParleyRaised`, rate-limited heartbeats, populated `RunFinished`, `WarEngine::trace_emitter()` (OBS-01)
- [x] 28-04-PLAN.md — `RunTracePort`, three storage adapters, `run_traces` migrations, contract suite, retention join (OBS-02)
- [x] 28-05-PLAN.md — `paladin-eval` crate, scenario file format, `ScenarioLlm`, golden JSON Schema (OBS-04)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 28-06-PLAN.md — Log sink, facade telemetry module, per-run composition, below-engine producers, bench + overhead evidence (OBS-02)
- [x] 28-07-PLAN.md — `GraphShape`, Mermaid/DOT exporters, five golden fixtures, `make bless-golden` (OBS-03)
- [x] 28-08-PLAN.md — Eval assertion library (twelve kinds) with snapshot-frozen failure rendering (OBS-04)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 28-09-PLAN.md — OTel sink: `otel` feature, span-per-attempt model, in-memory shape test, axum OTLP stub (OBS-02)
- [x] 28-10-PLAN.md — `ExecutionOverlay`, observed-only fallback, overlay Mermaid goldens (OBS-03)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 28-11-PLAN.md — SSE collapse to one producer, `trace_seq`, `RunStreamMode::Replay`, `PersistingTraceSink` (OBS-02)
- [x] 28-12-PLAN.md — Eval runner (`libtest-mimic` harness), `paladin-cli eval run` with repeat/bless/gated live mode (OBS-04)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 28-13-PLAN.md — CLI `graph export` and `run export` with snapshot tests (OBS-03)
- [x] 28-14-PLAN.md — `RunInspectorPort` and the facade inspector service (OBS-03)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 28-15-PLAN.md — `dev-ui` inspector page: first `paladin-web` feature, route, template, oneshot smoke test (OBS-03)
- [x] 28-16-PLAN.md — E2E fixture extraction and the three dogfood eval scenarios (OBS-04)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 28-17-PLAN.md — Docs, ADR-0048, `MIGRATION.md` rows, crate-list registration, close-out gates + CI evidence (all)

**UI hint**: yes

### Phase 29: Program Gates & Release

**Goal**: v0.10.0 is releasable — the migration record is complete, backward compatibility is proven rather than asserted, the program acceptance audit passes, and every crate publishes.
**Depends on**: Phase 22, Phase 23, Phase 24, Phase 25, Phase 26, Phase 27, Phase 28 (all)
**Requirements**: SHIP-01, SHIP-02, SHIP-03, SHIP-04
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `MIGRATION.md` has every §9 section filled with no "TBD" — M-B-01…03 resolved with chosen defaults and worked examples, the §9.2 register matching the `cargo semver-checks` allowlist exactly — and is linked from the README and the mdBook "Upgrading" page (SHIP-01)
  2. An integration test boots v0.10 with a v0.9 sample config and asserts legacy behavior (all new subsystems disabled by default), and a golden diff of `openapi.json` restricted to pre-existing paths is empty (SHIP-02)
  3. E2E-1/2/3 pass green as integration tests in `tests/`, the doc-08 verification protocol confirms every FR has a passing test with no orphan behavior and ubiquitous-language names conform, and BUG-01's old warn-and-default-true path is grep-absent with the fix's failing-then-passing test order visible in history (SHIP-03)
  4. All workspace crates are at `0.10.0` with changelogs updated, `cargo publish --dry-run` is green for every publishable crate in dependency order, mdBook + rustdoc are updated with no new broken intra-doc links, and the semver and MSRV CI jobs are green on the release commit (SHIP-04)

**Plans**: 9 plans

Plans:

**Wave 1** *(no dependencies — parallel)*

- [x] 29-01-PLAN.md — SHIP-02: frozen v0.9 config fixtures and the `v0_9_config_boot` proof (config resolution + 501 route table)
- [x] 29-02-PLAN.md — SHIP-02: path-restricted `$ref`-closure OpenAPI golden diff against the frozen v0.9.0 baseline
- [x] 29-03-PLAN.md — SHIP-01/SHIP-04: row-level allowlist ↔ §9.2 CI gate, and `publish-dry-run` + release-checklist corrections

**Wave 2** *(blocked on Wave 1)*

- [x] 29-04-PLAN.md — SHIP-03: acceptance-audit skeleton, per-FR evidence table, E2E/BUG evidence, orphan-behavior and language findings
- [x] 29-05-PLAN.md — SHIP-01/SHIP-02: close MIGRATION.md §9.5-§9.8 and the header, add the placeholder CI gate and the boot-test CI step

**Wave 3** *(blocked on Wave 2)*

- [x] 29-06-PLAN.md — SHIP-01: the mdBook Upgrading page, SUMMARY entry, migration-guide pointer, overview §4 errata
- [x] 29-07-PLAN.md — SHIP-03: audit steps 6-9, the accepted tracing-overhead deviation, the maintainer sign-off checklist

**Wave 4** *(blocked on Wave 3)*

- [x] 29-08-PLAN.md — SHIP-03: WINDOWS.md triage — 25 open rows dispositioned with citations, plus the overhead-deviation row

**Wave 5** *(blocked on Wave 4)*

- [x] 29-09-PLAN.md — SHIP-04: the 0.10.0 bump, changelogs, dry-run publish, CI evidence, audit step 10

### Phase 30: Token-Economy Vocabulary & Commissary Anchoring

**Goal**: The token-economy vocabulary is decided in writing and `Commissary` has a documentation home — the units-plain / roles-medieval rule is recorded, `Commissary` is anchored with an on-branch ADR and an mdBook page, `Treasurer` is reserved (not built) with its downstream guardrail, the four meanings of `max_tokens` are documented, the last orphan `Quartermaster` references are gone, and the clean-break versioning decision that Phases 31-33 rely on is an ADR rather than an assumption.
**Depends on**: Phase 29 (docs-only; can land alone and first)
**Requirements**: VOCAB-01, VOCAB-02, VOCAB-03, VOCAB-04, VOCAB-05, VOCAB-06, VOCAB-07
**Source**: `.project/Milestone_13-Token-Economy/Epic_1/prd-vocabulary-and-docs-foundation.md` (D-1, D-2, D-3, D-8, D-9; F4 partial, F5 docs)
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `PROJECT.md` and `docs/src/architecture/domain-model.md` state the vocabulary rule — units/measures (`TokenUsage`, `max_tokens`, `max_context_tokens`) and technical ports (`TokenCounterPort`, `LlmPort`, `EmbeddingPort`) keep plain names; domain roles, places and events get Medieval-Military names — and `Commissary` appears in both the ubiquitous-language list and the domain-model table as the input-side, per-call window-rationing officer (VOCAB-01)
  2. A numbered ADR in `.planning/decisions/` records the `Commissary` design (`verify_fits` guard + `dispense` allocator, fail-loud / never-silent), the Quartermaster→Commissary rename rationale and the explicit rejected-name list, reconstructed from `origin/feature/quartermaster-prompt-budgeting`'s `0010-prompt-context-budgeting.md` and the port commit history — and an mdBook page under `docs/src/` (concept, the `Consignment`/`Stockpile`/`ShedItem` model, a usage sketch) is reachable from the architecture nav with the link-check green (VOCAB-02, VOCAB-03)
  3. A one-page `Treasurer` reservation ADR exists: the role is reserved (0/0 in-tree, verified by grep), will own cross-run / per-tenant / per-API-key allowances, per-model currency pricing, `cost_estimate` production and rate pacing, *installs* a per-run `TokenBudget` rather than replacing it, is built in a later milestone (Milestone 14 in `.project/Milestone_14-Treasurer/`), and — the downstream guardrail — is a framework-only word that must never appear as an audit-target or fixture domain term (VOCAB-04)
  4. `docs/src/getting-started/configuration.md` carries one table naming the four `max_tokens` meanings (Garrison store cap, RAG injection cap, per-request completion cap, run-level `token_budget` cap) and states that any future Treasurer-level cap uses a distinct key (`allowance`); the rustdoc on `ExecutionMetadata.cost_estimate` (`crates/paladin-core/src/platform/container/herald.rs`) says it is reserved for the Treasurer (Milestone 14 / FUT-08) with no in-tree producer, and the field is not removed (VOCAB-05)
  5. `grep -rniE '\bQuartermaster\b' crates src` returns nothing — the `src/lib.rs` provenance comment is reworded without the retired term — and the `SirQuartermaster` example in `.project/project-management/paladin-project-plan-final.md` is annotated as historical; `.planning/` phase history is untouched (VOCAB-06)
  6. A token-economy versioning ADR records that Phases 31-33 land as **clean breaks inside the untagged v0.10.0** — superseding, for these phases only, the v0.10.0 corpus rule X-03 ("deprecations allowed, removals are not, before v0.11.0") on the operator's 2026-09-14 decision (single coordinated downstream consumer, pre-1.0) — and that every break still gets a `MIGRATION.md` §9.2 row and a `cargo semver-checks` allowlist row as documentation for the downstream refactor, never as a compatibility shim; the supersession is also recorded in `PROJECT.md` Key Decisions (VOCAB-07)

**Plans**: 3 plans

Plans:
**Wave 1**

- [x] 30-01-PLAN.md — Anchor `Commissary`: ADR-0049 (design, rename rationale, nine rejected names), the vocabulary rule in all three ubiquitous-language lists, and the new `docs/src/architecture/commissary.md` page linked from the architecture nav (VOCAB-01, VOCAB-02, VOCAB-03) — wave 1

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 30-02-PLAN.md — ADR-0050 (`Treasurer` reservation + downstream guardrail) and ADR-0051 (clean breaks inside the untagged v0.10.0, superseding X-03 for Phases 31-33), plus the PROMOTION.md index and PROJECT.md Key Decisions bookkeeping (VOCAB-04, VOCAB-07) — wave 2

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 30-03-PLAN.md — The four meanings of `max_tokens` as one table, the `cost_estimate` rustdoc reservation at five doc sites, and the last two `Quartermaster` prose references retired from `crates`/`src` (VOCAB-05, VOCAB-06) — wave 3

### Phase 31: Lossless Token Accounting

**Goal**: The provider's prompt/completion split survives from the LLM port to `RunFinished` and a herald — no carrier above the port collapses `TokenUsage` to a bare total any more, `TokenUsage` gains optional cache and reasoning fields, and streamed runs report the same usage as non-streamed ones — so that everything cost-shaped (currency pricing, the Treasurer) becomes buildable on the shipped shape. This is the keystone phase: Phase 32 depends on it and Milestone 14 will.
**Depends on**: Phase 29 (keystone — independent of Phase 30; Phase 30's VOCAB-07 versioning ADR should land first so the clean break is a recorded decision)
**Requirements**: ACCT-01, ACCT-02, ACCT-03, ACCT-04, ACCT-05
**Source**: `.project/Milestone_13-Token-Economy/Epic_2/prd-lossless-token-accounting.md` (D-4; F1, F8)
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `TokenUsage` (`crates/paladin-core/src/platform/container/token_usage.rs`, the single definition) carries `cache_read_tokens`, `cache_write_tokens` and `reasoning_tokens` as `#[serde(default)]` optionals; the rustdoc states whether `total_tokens` includes them; a legacy JSON document without the new fields deserializes via defaults and a new document round-trips (ACCT-01)
  2. `PaladinResult`, `BattalionResult.per_paladin_tokens`, the Waypoint `NodeExecutionRecord`, `TraceEvent::NodeFinished` and `RunFinished` carry a full `TokenUsage` rather than a bare count; `TokenUsage::from_total` is gone from the battalion aggregation path (`formation_service.rs`, `phalanx_service.rs`), and a round-trip test proves a usage with non-zero prompt AND completion (plus cache/reasoning) reaches `RunFinished` intact — a clean break with no `#[deprecated]` bare-count accessor whose only purpose is downstream compatibility (ACCT-02)
  3. Every LLM adapter's `execute_stream` path is audited: a per-adapter test asserts the accumulated `TokenUsage` on the streaming path equals the non-streaming path, or the adapter's inability to report streamed usage is documented as an explicit exception in the adapter's rustdoc and the mdBook provider page (ACCT-03)
  4. The breakdown is observable end-to-end in at least one herald in both JSON and Markdown output (ACCT-04)
  5. Every touched public type has a `MIGRATION.md` §9.2 row and a matching `cargo semver-checks` allowlist row (Phase 29 D-04 row-level CI gate green), the `CHANGELOG.md` `[0.10.0]` section records the carrier change, and `make clean-code` plus the 82 % coverage floor are green (ACCT-05)

**Plans**: 7 plans

Plans:
**Wave 1**

- [x] 31-01-PLAN.md — `TokenUsage` gains the three optional sub-counts, saturating `Add`/`AddAssign`/`Sum` and builders; every in-tree literal migrates (ACCT-01)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 31-02-PLAN.md — the carrier break: `PaladinResult`/`NodeExecutionRecord`/`NodeFinished`/`RunFinished` carry a full `TokenUsage`, the total-only constructor is deleted, real per-Paladin splits (ACCT-02)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 31-03-PLAN.md — streaming terminal-chunk usage contract on the port plus `CompatEngine`/OpenAI/DeepSeek, and the no-estimation fallback (ACCT-03)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 31-04-PLAN.md — Anthropic and Gemini streaming usage, the shared parity conformance case, and the documented provider exception (ACCT-03)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 31-05-PLAN.md — JSON and Markdown herald breakdown, CLI split rendering, and the mdBook/rustdoc sweep (ACCT-04)
- [x] 31-06-PLAN.md — HTTP edge: `TokenUsageResponse`, `ExecuteResponse.usage`, inspector `CompletedRow.usage`, regenerated `openapi.json` (ACCT-02, ACCT-05)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 31-07-PLAN.md — `MIGRATION.md` §9.2 rows, empirically-derived semver allowlist entries, `CHANGELOG.md` `[0.10.0]`, and the phase gate evidence (ACCT-05)

### Phase 32: Unified Token Primitives

**Goal**: Exactly one token-counting contract and exactly one context-window resolver exist — `TokenCounterPort` gains its exactness signal so `Commissary::new` stops asking the caller for it, the legacy fallible `garrison::TokenCounter`/`TokenCounterFactory` pair is retired, and `HistoryTrimmer` and `Commissary` resolve the window through a single shared function that preserves Commissary's strict "no invented window" refusal.
**Depends on**: Phase 31 (the primitives layer settles together), Phase 30 (VOCAB-07 versioning ADR)
**Requirements**: PRIM-01, PRIM-02, PRIM-03, PRIM-04, PRIM-05
**Source**: `.project/Milestone_13-Token-Economy/Epic_3/prd-unify-token-primitives.md` (D-5, D-6; F2, F3)
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `TokenCounterPort` (`crates/paladin-ports/src/output/token_counter_port.rs`) has `fn is_exact(&self) -> bool` defaulting to `false`; the tiktoken-backed counter returns `true` and the heuristic counter returns `false`, each proven by a test (PRIM-01)
  2. `Commissary::new` no longer takes an `is_exact_counter: bool` argument and reads exactness from the port — a clean break with no forwarding constructor — and every in-tree call site compiles against the new signature (PRIM-02)
  3. The legacy `garrison::TokenCounter` trait and `TokenCounterFactory` are removed together with their three re-exports (`paladin-memory` `garrison/mod.rs` and `prelude.rs`, and the facade's `src/infrastructure/adapters/garrison/mod.rs`), every former in-tree caller consuming `TokenCounterPort` instead; if one internal caller genuinely cannot migrate, it is marked `#[deprecated]` with the blocking reason recorded in the phase context and removal assigned to Phase 33 (PRIM-03)
  4. A shared resolver in `paladin-llm` (e.g. `window::resolve_context_window`) owns the precedence config table → provider capabilities → default, with an explicit strict mode that errors rather than defaults when the window is unknown; both `HistoryTrimmer` (`src/application/services/paladin/middleware/history.rs`, today's `resolve_limit`) and `Commissary` consume it; precedence tests cover config-table hit, provider-capability hit, default fallback and strict refusal, and an equivalence snapshot proves `Commissary` resolves the same windows as before this phase and `HistoryTrimmer` produces the same trims (PRIM-04)
  5. The `Commissary::new` signature change and the legacy-counter removal each have a `MIGRATION.md` §9.2 row and a semver-checks allowlist row (row-level gate green), the `CHANGELOG.md` `[0.10.0]` section records them, and `make clean-code` plus the coverage floor are green (PRIM-05)

**Plans**: 5 plans

Plans:

**Wave 1**

- [x] 32-01-PLAN.md — `TokenCounterPort::is_exact` defaulted `false` with tiktoken `true` and heuristic inheriting it, `Commissary` dropping its caller-supplied exactness argument and reading the port, plus the consolidated one-way checkpoint for the phase (PRIM-01, PRIM-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 32-02-PLAN.md — the pre-resolver equivalence snapshot committed green, then `paladin_llm::window`: one precedence walk with an explicit fallback-policy enum, a labelled source enum and four precedence tests (PRIM-04)
- [x] 32-03-PLAN.md — legacy `garrison::TokenCounter`/`TokenCounterFactory` deleted outright, the tiktoken counting path inlined into the port impl, four re-export sites narrowed and the doc sweep with its exit grep (PRIM-03)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 32-04-PLAN.md — both consumers call the shared resolver: `Commissary::new` under the strict policy with an absent table, `HistoryTrimmer::resolve_limit` under the lenient policy, the facade's duplicate source enum deleted, equivalence fixtures green and unedited (PRIM-04)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 32-05-PLAN.md — empirical semver discovery with the mandatory `--release-type minor` and the feature-gated second pass, `MIGRATION.md` §9.2 rows with row-matched allowlist entries, `CHANGELOG.md` `[0.10.0]` bullets, the two migration pages, and the phase gate evidence (PRIM-05)

### Phase 33: Commissary In-Tree Adoption

**Goal**: `Commissary` has a real production caller and the last silent token-truncation path is gone — RAG retrieval rations its injection budget through `Commissary::dispense` with score-derived priorities, records every shed memory and marks truncated output — and, because Phases 31-33 changed public API after Phase 29 sealed the release gates, those gates are re-run green on the final commit so v0.10.0 is releasable again.
**Depends on**: Phase 32 (shared resolver, settled counter contract), Phase 31, Phase 30
**Requirements**: COMM-01, COMM-02, COMM-03, COMM-04
**Source**: `.project/Milestone_13-Token-Economy/Epic_4/prd-commissary-in-tree-adoption.md` (D-7; F6, F4 completes)
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `RagRetrievalService::truncate_to_token_budget`'s inline `content.len() / 4` + silent drop (`crates/paladin-memory/src/services/rag_retrieval_service.rs`) is replaced by a `Commissary::dispense` call over a `Consignment` built from the retrieved memories with priority derived from relevance score and budget `rag.max_tokens`; a property test proves the retained set's total is ≤ the budget and the highest-scoring memories are the ones retained (COMM-01)
  2. The `ShedItem` list (which memories were dropped and why) is surfaced through the RAG result path, and a truncation marker is emitted when content was shed; tests assert both are present when the budget is exceeded and both absent when everything fits (COMM-02)
  3. An integration test exercises `Commissary::dispense` through the real RAG path — the F4 production-caller evidence — and no silent token-based truncation remains in-tree (the Phase 26 D-13 deferral closes; grep-provable) (COMM-03)
  4. The Phase 29 release gates are re-sealed on the final commit of this phase: `MIGRATION.md` has no "TBD" and its §9.2 register matches the semver-checks allowlist row-for-row, `v0_9_config_boot` and the OpenAPI golden diff pass, `cargo semver-checks` and the MSRV job are green, `cargo publish --dry-run` is green in dependency order, and the `CHANGELOG.md` `[0.10.0]` section carries the RAG truncation-marker behavioural note plus the Phase 31/32 API entries — with the evidence appended to the Phase 29 acceptance audit rather than a new audit (COMM-04)

**Plans**: 6 plans

Plans:

**Wave 1**

- [x] 33-01-PLAN.md — the `paladin-memory` → `paladin-llm` production edge plus the tracer slice: result struct, error enum, the sync `ration` seam and the first `Commissary::dispense` call, with every consumer migrated in one commit (COMM-01)

**Wave 2** *(blocked on Wave 1)*

- [x] 33-02-PLAN.md — one shared omission-marker helper emitted by both renderers, the counts-only observability line, and the both-directions marker/shed tests (COMM-02)

**Wave 3** *(blocked on Wave 2; 33-03 and 33-04 run in parallel)*

- [x] 33-03-PLAN.md — the `proptest` over the rationing seam plus the four named edge tests: oversized-single-memory, non-clamping budget conversion, equal-score tie order, budget boundary (COMM-01)
- [x] 33-04-PLAN.md — the ungated `rag_commissary` integration test (F4 evidence), the Commissary module-doc retirement and the D-19 exit greps (COMM-03)

**Wave 4** *(blocked on Wave 3)*

- [x] 33-05-PLAN.md — empirical semver discovery, the two `MIGRATION.md` §9.2 rows with row-matched allowlist entries, the `CHANGELOG.md` `[0.10.0]` entries with the `[Unreleased]` fold, and the regenerated API baseline (COMM-04)

**Wave 5** *(blocked on Wave 4 — runs on the phase's final commit)*

- [x] 33-06-PLAN.md — the full Phase 29 gate re-seal with the PRIM-04 regression check, `33-CI-EVIDENCE.md`, and audit §11 with one unticked human-only tag box (COMM-04)

### Phase 34: Documentation Currency Audit

**Goal**: The documentation debt is measured before it is paid — one inventory records, per mdBook page under `docs/src/`, per crate's rustdoc, and per `examples/` / `crates/doc-examples` program, what Phases 22-33 changed that the docs do not yet say (plus any v0.9.0-era gap the Phase 16 currency pass and the Phase 28-17 / 29-06 docs plans left open), with every finding classified as *missing page*, *stale content*, *rustdoc warning or broken intra-doc link*, or *non-compiling / obsolete example*, so that Phases 35 and 36 are scoped by evidence rather than by guess.
**Depends on**: Phase 33 (the tree the docs must describe is final — all 13 phases of the milestone are verified)
**Requirements**: CURR-01, CURR-02, CURR-03, CURR-04, CURR-05 (minted at planning 2026-09-17; prefix shared with Phases 35-36)
**Source**: Operator instruction 2026-09-17 (pre-tag readiness review); `33-CI-EVIDENCE.md` row 26 (73 carried `cargo doc` warnings); STATE.md Phase 32 close (14 unresolved intra-doc links under `--all-features`); `WINDOWS.md` #36
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. A single audit document in the phase directory lists every mdBook page under `docs/src/` with a currency verdict (current / stale / missing) against the Phase 22-33 shipped surface, and every stale or missing verdict cites the phase and the shipped item (type, route, config key, CLI subcommand) the page fails to describe
  2. The rustdoc failures are enumerated, not summarised: every `warning:` line from `cargo doc --workspace --no-deps` and every unresolved intra-doc link from the `RUSTDOCFLAGS="-D warnings" … --all-features` run is listed with crate, file and line, and the `ci.yml` lint-job "Check documentation" command is quoted verbatim as the bar Phase 36 must clear
  3. Every program under `examples/` and every module of `crates/doc-examples` is recorded with its build status under the feature sets the CI `cargo build --examples` step splits on, and a currency verdict — which Phase 22-33 API it should demonstrate but does not, or which removed / renamed API it still names
  4. The inventory is partitioned into the Phase 35 (mdBook) and Phase 36 (rustdoc + examples) work lists with each item sized, and anything found that is neither documentation nor an example is routed to the deferred register rather than absorbed into either phase
  5. No documentation, rustdoc or example is changed in this phase — the audit is read-only against the tree, and the phase's commits touch only `.planning/`

**Plans**: 9 plans

Plans:
**Wave 1**

- [x] 34-01-PLAN.md — Tracer: mint CURR-01…05, build the 34-AUDIT.md spine, prove one worked row per table

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 34-02-PLAN.md — Shipped-surface checklist (§1) compiled from CHANGELOG / MIGRATION / REQUIREMENTS

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 34-03-PLAN.md — mdBook build + linkcheck + vocabulary baseline, and the 18 root/getting-started/architecture/api-reference verdicts

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 34-04-PLAN.md — mdBook verdicts for the 20 user-guides and the 20 deployment/topologies/operations/contributing pages

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 34-05-PLAN.md — mdBook verdicts for the 34 remaining appendix pages; the 93-page partition closes

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 34-06-PLAN.md — Rustdoc default-feature enumeration against the ci.yml bar, plus the workspace all-features record

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 34-07-PLAN.md — Per-crate all-features sweep, doctest baseline, public-API example-heading gate record

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 34-08-PLAN.md — Examples build status under the four CI feature sets, currency verdicts and the capability gap list

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 34-09-PLAN.md — Phase 35 / Phase 36 work lists, deferred register, and the phase-range read-only proof

### Phase 35: mdBook Currency

**Goal**: The mdBook describes the v0.10.0 tree — every gap the Phase 34 inventory records for `docs/src/` is closed: a page exists for each Phase 22-33 capability that shipped without one, every stale page is corrected to the shipped API and vocabulary (the superstep engine, Parley, Aegis, the platform API, the `TokenUsage` split, `Commissary`), the Upgrading page and migration pointers agree with `MIGRATION.md`, and `mdbook build` with the linkcheck backend is green.
**Depends on**: Phase 34 (the mdBook work list); independent of Phase 36 and may run in parallel with it
**Requirements**: CURR-06, CURR-07, CURR-08, CURR-09, CURR-10 (minted at planning 2026-09-17;
prefix shared with Phases 34 and 36)
**Source**: Phase 34 audit inventory (mdBook partition); `.github/workflows/docs.yml`
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. Every item in the Phase 34 mdBook work list is closed by a page edit or a new page, and `docs/src/SUMMARY.md` links each new page from the nav position the audit assigned
  2. `mdbook build docs/` with the `linkcheck` backend passes with zero broken links — the exact `docs.yml` command sequence, including `mdbook-mermaid install`
  3. No touched page names a type, function, config key, route or CLI flag the v0.10.0 tree does not export; snippets meant to run are compile-verified in `crates/doc-examples`, and illustrative snippets are marked as such
  4. The book's vocabulary matches the three ubiquitous-language lists (Phase 30 VOCAB-02): no `Quartermaster`, and no bare token total where the prompt / completion split shipped in Phase 31
  5. `CHANGELOG.md` `[0.10.0]` carries a Documentation entry summarising the pages added and corrected

**Plans**: 10 plans

Plans:

**Wave 1**

- [x] 35-01-PLAN.md — Tracer: mint CURR-06…CURR-10, write the WarEngine superstep-engine guide with its compile-verified doc-examples module and nav entry, seed the deferred register (MB-30)

**Wave 2** *(blocked on Wave 1 — eight plans with disjoint file sets)*

- [x] 35-02-PLAN.md — Five new `doc-examples` modules for the signature-level user-guide rows (MB-19, MB-20, MB-24, MB-27, MB-28)
- [x] 35-03-PLAN.md — Getting Started and User Guides version/MSRV/feature sweep plus the three line-pinned content fixes (MB-06, MB-07, MB-18, MB-21, MB-22, MB-23, MB-25, MB-26, MB-29)
- [x] 35-04-PLAN.md — Introduction and Architecture: vocabulary close-out, domain-model entities, crate and API-shape corrections (MB-02, MB-03, MB-04, MB-05, MB-08, MB-09, MB-10, MB-11, MB-12)
- [x] 35-05-PLAN.md — API Reference and Contributing: the ADR index retitle-and-add, both crate maps, feature flags, migration guide, stable API (MB-13, MB-14, MB-15, MB-16, MB-17, MB-35)
- [x] 35-06-PLAN.md — CI pages rebuilt on the real job inventory and the three superseded Operations callouts (MB-31, MB-32, MB-33, MB-34, MB-36)
- [x] 35-07-PLAN.md — The CLI family rebuilt from live `--help` captures (MB-40, MB-41, MB-42, MB-43, MB-44, MB-45, MB-46)
- [x] 35-08-PLAN.md — Appendix archive tier plus three correct-tier snapshot pages (MB-01, MB-37, MB-39, MB-47, MB-54, MB-55, MB-59, MB-60)
- [x] 35-09-PLAN.md — Appendix import-path family and the API-shape/inventory pages (MB-38, MB-48, MB-49, MB-50, MB-51, MB-52, MB-53, MB-56, MB-57, MB-58)

**Wave 3** *(blocked on Wave 2 — runs on the phase's final commit)*

- [x] 35-10-PLAN.md — `35-EVIDENCE.md` with the sixty-row closure table and the full gate run, the `[0.10.0]` Documentation changelog entry, the D-21 exit greps and the deferred-register fold

### Phase 36: Rustdoc Zero-Warning Bar & Examples Currency

**Goal**: The rustdoc corpus clears the bar CI already enforces and the examples demonstrate the tree that ships — `cargo doc --workspace --no-deps` emits zero `warning:` lines so the lint job's "Check documentation" step is green rather than carried, the 14 unresolved intra-doc links under `--all-features` are resolved, every public item Phases 22-33 added or changed has rustdoc (with a doc test where the project's public-API rule applies), and every `examples/` program and `crates/doc-examples` module builds against and demonstrates the v0.10.0 API.
**Depends on**: Phase 34 (the rustdoc + examples work list); independent of Phase 35 and may run in parallel with it
**Requirements**: CURR-11, CURR-12, CURR-13, CURR-14, CURR-15
**Source**: Phase 34 audit inventory (rustdoc + examples partition); `33-CI-EVIDENCE.md` row 26; `WINDOWS.md` #36; `.github/workflows/ci.yml` lint job and the `cargo build --examples` feature-set split
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. `cargo doc --workspace --no-deps` emits zero `warning:` lines under the exact `ci.yml` lint-job command, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exits 0, closing `WINDOWS.md` #36
  2. Every Phase 34 rustdoc finding is closed at its cited crate / file / line, and both rustdoc commands are added to the pre-push gate or `make clean-code` so the warning count cannot silently regrow
  3. `cargo build --examples` passes under each feature set the CI step splits on, and `cargo test --workspace --doc` is green — run explicitly, because the coverage and `--tests` gates skip doctests
  4. Every Phase 34 example finding is closed: obsolete examples are updated to the shipped API or deleted with a `CHANGELOG.md` note, and each Phase 22-33 capability the audit flagged as undemonstrated has a runnable example listed in `examples/README.md`
  5. `make api-surface` reports no change — docs and examples do not move the public surface; if a fix genuinely requires a public change it is recorded in `MIGRATION.md` §9.2 and the semver allowlist per the Phase 29 / 33 pattern

**Plans**: 13 plans

Plans:
**Wave 1**

- [x] 36-01-PLAN.md — Tracer: close the memory/ports/storage rustdoc groups, add the token-economy example, seed the evidence harness

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 36-02-PLAN.md — Rustdoc: paladin-battalion (72 rows, 34 location groups)
- [x] 36-03-PLAN.md — Rustdoc: paladin-ai-core (28 rows, 14 location groups)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 36-04-PLAN.md — Rustdoc: paladin-llm and paladin-web (24 rows, 17 location groups)
- [x] 36-05-PLAN.md — Rustdoc: the paladin-ai facade (12 rows, 7 location groups)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 36-06-PLAN.md — Examples: WarEngine configuration & checkpoints, control flow & dynamic routing
- [x] 36-07-PLAN.md — Examples: human-in-the-loop gate & resume, graceful shutdown

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 36-08-PLAN.md — Examples: agent runtime & middleware, structured output, RAG retrieval
- [x] 36-09-PLAN.md — Examples: http_service_host router parity, Platform API client, webhook receiver

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 36-10-PLAN.md — Examples: node-result cache, observability & OTel export, eval scenarios

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 36-11-PLAN.md — examples/README.md: gallery index completion and currency fixes
- [x] 36-12-PLAN.md — Gate wiring: make doc-check, clean-code, pre-push, CI lint step, examples check, closing measurement

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 36-13-PLAN.md — Closure map, WINDOWS.md rows 36/37, CHANGELOG entries, CI evidence

**Cross-cutting constraints:**

- `make api-surface` reports the surface unchanged.
- `cargo check --workspace --all-targets --all-features` exits 0 and `cargo test --workspace --doc` stays green.
- Both programs are picked up by the bulk `cargo build --examples` selector — neither declares required-features.
- `cargo check --workspace --all-targets --all-features` exits 0 and `make api-surface` reports the surface unchanged.

### Phase 36.1: Deferred Items Closure (INSERTED)

**Goal**: Every deferred item Phases 30-35 recorded and left unowned is either closed in the tree or explicitly dispositioned before v0.10.0 ships — the per-phase `deferred-items.md` registers (31, 32, 34, 35), the two open `WINDOWS.md` rows (#36, #37) and the two `todos/pending/` files are walked item by item, each one is fixed, waived with a written reason, or re-homed to a named owner, and the `WINDOWS.md` ledger is brought back into agreement with the registers so `/gsd-complete-milestone` sees the whole picture rather than two rows.
**Depends on**: Phase 36 (closes the rustdoc items — `WINDOWS.md` #36, #37 and the 73-warning `cargo doc` baseline — that this phase must verify closed rather than fix twice); independent of Phase 35
**Requirements**: CURR-16, CURR-17, CURR-18, CURR-19, CURR-20, CURR-21
**Source**: `phases/31-lossless-token-accounting/deferred-items.md`, `phases/32-unified-token-primitives/deferred-items.md`, `phases/34-documentation-currency-audit/deferred-items.md`, `phases/35-mdbook-currency/deferred-items.md`; `WINDOWS.md` rows 36-37; `todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, `todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`; STATE.md Phase 32 / 33 close-out carried concerns
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. The unowned `docs/src` prose defects Phase 35 deferred are closed on the page: `contributing-providers.md` lines 272 and 367 use the relocated adapter path, `testing-guide.md`'s `tests/` tree no longer places `config.test.yml` under `fixtures/`, `cli-configuration.md`'s Garrison and Arsenal troubleshooting entries no longer assert a source-line TODO, `grep -rnw OpenAiAdapter docs/src` is empty across all seven pages, and `cicd.md`'s deploy and best-practice YAML is either captioned illustrative or replaced by a real workflow excerpt
  2. The Phase 34 tooling findings are dispositioned, not merely re-pointed: `scripts/check-public-api-examples.sh` is either wired into CI or `make clean-code` against a refrozen entry-point baseline with its 19 MISSING items fixed, or waived in `WINDOWS.md` with the maintainer's reason; `ci.yml`'s examples-count comment matches `find examples -name '*.rs' | wc -l`; PROJECT.md's Phase 4 amendment names `paladin-llm` as the one crate with its own `examples/`
  3. `tests/cli_isolation_test.rs::test_cli_feature_is_not_default` no longer fails under `cargo test --workspace --all-features` — gated or rescoped — so the three-phase-old carried failure stops being re-logged
  4. `WINDOWS.md` #36 and #37 are `fixed` (verified against Phase 36's output, not re-done here), and every open entry from the Phase 34 and 35 registers has a `WINDOWS.md` row with status `fixed` or `waived` plus a reason, so the ledger and the registers agree
  5. The two `todos/pending/` items (local coverage reproduction on a Docker machine; RustFS evaluation) are each either completed, or explicitly deferred past v0.10.0 with an owner and a re-check date written into the todo file — neither is left as an undated pointer
  6. `make clean-code`, `make security`, `cargo test --workspace`, `mdbook build` with linkcheck, and `make api-surface` are green on the closing commit; docs-only fixes move no public surface

**Plans**: 14 plans

Plans:

**Wave 1**

- [x] 36.1-01-PLAN.md — Tracer: rewrite the `cli_isolation` guard against the manifest, seed the closure table and the coverage declaration, capture the all-features sweep

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 36.1-02-PLAN.md — Sanitize the tool-failure reason on both run-failing arms, with a pinning test (ledger row 38)
- [ ] 36.1-03-PLAN.md — SC1 pages: provider-guide adapter path, CLI troubleshooting entries, illustrative CI/CD captions
- [ ] 36.1-04-PLAN.md — SC1 pages: rebuild the tests tree, sweep the adapter type casing across seven pages, run the docs gate
- [ ] 36.1-05-PLAN.md — `# Examples` for six lighter port traits
- [ ] 36.1-06-PLAN.md — `# Examples` for the trace port and the two assistant ports
- [ ] 36.1-07-PLAN.md — `# Examples` for the three widest port traits; all twelve ports satisfied
- [ ] 36.1-08-PLAN.md — `# Examples` for `ScheduleService` and `WebhookDeliveryService`
- [ ] 36.1-09-PLAN.md — `# Examples` for `RunSubmissionService` and `RunInspectorService`
- [ ] 36.1-10-PLAN.md — `# Examples` for `RunEventStreamService` and `AssistantService`

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 36.1-11-PLAN.md — The nineteenth section and the three-place gate wiring in one commit; the refrozen entry-point snapshot and its two pointers

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 36.1-12-PLAN.md — PROJECT.md corrections, the ADR-0033 suppressions amendment, both todo dispositions, the v2 candidate line, the changelog bullets

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 36.1-13-PLAN.md — Twelve ledger rows plus row 38, the completed closure table, the closing gate sequence

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 36.1-14-PLAN.md — CI evidence record and the blocking push checkpoint

**Cross-cutting constraints:**

- `make api-surface` reports the surface unchanged on every commit — the only visibility change is `pub(crate)`, which `cargo public-api` never lists.
- Nothing under `src/` or `crates/` changes except the two tool-error match arms, the sanitizer's visibility, their test, the nineteen doc comments and `tests/cli_isolation_test.rs`.
- `.planning/WINDOWS.md` is mutated only through the ledger CLI; no row is ever deleted.
- The gate wiring lands in the same commit as the nineteenth `# Examples` fix, so no commit exists where the gate is wired and red.
- The executor never pushes — the branch push is the maintainer's action at the 36.1-14 checkpoint.

### Phase 37: v0.10.0 Crate Release

**Goal**: v0.10.0 is released, not merely releasable — the Phase 29 gates are re-sealed on the final post-documentation commit, the feature branch merges to `main`, `release.yml` cuts the `v0.10.0` tag on the merge commit per the Phase 29 two-SHA rule, every publishable crate is on crates.io at `0.10.0`, and the release evidence is recorded so the milestone can close.
**Depends on**: Phase 35, Phase 36 (all documentation and example work landed); Phase 36.1 (deferred items closed or dispositioned before the tag); Phase 33 (the gate re-seal this phase repeats)
**Requirements**: TBD — assigned at planning; may extend SHIP-04 in place per protocol item 3 rather than minting a near-duplicate
**Source**: Phase 29 D-17 / D-18 / D-21 (human-only §11 sign-off box; `0.10.0` bumped without a tag; tag cut on the `main` merge commit by `release.yml`); the v0.9.0 post-close release record in MILESTONES.md
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. The Phase 29 gate set — `MIGRATION.md` with no "TBD" and its §9.2 register matching the semver-checks allowlist row-for-row, `v0_9_config_boot`, the OpenAPI golden diff, `cargo semver-checks`, the MSRV job, `make publish-dry-run` in dependency order, and a complete `CHANGELOG.md` `[0.10.0]` — is re-run green on the final commit, with the evidence appended to the Phase 29 acceptance audit and the §11 human sign-off box ticked by the maintainer
  2. The CI `coverage` job on the pre-merge run reports at or above the ADR-0006 floor and the run is recorded in the CI-evidence table — the one gate this devcontainer cannot measure locally
  3. The feature branch is merged to `main`, `release.yml` runs green, and the `v0.10.0` tag sits on the merge commit
  4. Every publishable crate resolves on crates.io at `0.10.0` (the `publish = false` `doc-examples` crate excluded), verified against the registry index and recorded in MILESTONES.md alongside the v0.9.0 entry
  5. The milestone is closed after the tag via `/gsd-complete-milestone v0.10.0`: the `## Milestones` row flips to Shipped, phase detail archives to `milestones/v0.10.0-ROADMAP.md`, and the next milestone starts at Phase 38

**Plans**: 0 plans

Plans:

- [ ] TBD (run /gsd-plan-phase 37 to break down)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|---|---|---|---|---|
| 1-4 | v0.7.1 | 38/38 | ✅ Shipped | 2026-08-04 |
| 5. Milestone 2-3 Ground Truth | v0.8.0 | 13/13 | ✅ Complete | 2026-08-05 |
| 6. Verified Gap Closure | v0.8.0 | 10/10 | ✅ Complete | 2026-08-05 |
| 7. Workspace Ground Truth & Recorded Answers | v0.8.0 | 13/13 | ✅ Complete | 2026-08-06 |
| 8. Verified Defect Closure | v0.8.0 | 9/9 | ✅ Complete | 2026-08-07 |
| 9. Release & Security Gate Integrity | v0.8.0 | 7/7 | ✅ Complete | 2026-08-08 |
| 10. Milestone 7-8 Ground Truth & Recorded Account | v0.8.0 | 11/11 | ✅ Complete | 2026-08-08 |
| 11. Facade Residue & Deferred Register Disposition | v0.8.0 | 5/5 | ✅ Complete | 2026-08-09 |
| 12. Supply-Chain Gate Integrity | v0.8.0 | 4/4 | ✅ Complete | 2026-08-10 |
| 13. Milestone 9-12 Ground Truth & Recorded Account | v0.8.0 | 13/13 | ✅ Complete | 2026-08-10 |
| 14. API Contract Truthfulness | v0.8.0 | 8/8 | ✅ Complete | 2026-08-12 |
| 15. Coverage & CI Quality Gates | v0.8.0 | 10/10 | ✅ Complete | 2026-08-13 |
| 15.1 Git & CI Governance (INSERTED) | v0.8.0 | 10/10 | ✅ Complete | 2026-08-14 |
| 16. Documentation Currency & the Architecture Gap | v0.8.0 | 14/14 | ✅ Complete | 2026-08-24 |
| 17. Additional LLM Provider Adapters | v0.8.0 | 22/22 | ✅ Complete | 2026-08-23 |
| 18-21 | v0.9.0 | 25/25 | ✅ Shipped | 2026-09-01 |
| 22. Battlefield State & Superstep Engine | v0.10.0 | 17/17 | Complete    | 2026-09-02 |
| 22.1 Engine readiness defect and MSRV follow-up (INSERTED) | v0.10.0 | 7/7 | Complete    | 2026-09-03 |
| 23. Control Flow — Dynamic Routing, Fan-Out & Subgraphs | v0.10.0 | 12/12 | Complete    | 2026-09-04 |
| 24. Pause/Resume, History & Graceful Shutdown | v0.10.0 | 14/14 | Complete    | 2026-09-05 |
| 25. Node-Level Fault Tolerance | v0.10.0 | 14/14 | Complete    | 2026-09-06 |
| 26. Agent Runtime Enhancements | v0.10.0 | 21/21 | Complete    | 2026-09-07 |
| 27. Platform API | v0.10.0 | 26/26 | Complete    | 2026-09-08 |
| 28. Observability & Tooling | v0.10.0 | 17/17 | Complete    | 2026-09-09 |
| 29. Program Gates & Release | v0.10.0 | 9/9 | Complete    | 2026-09-10 |
| 30. Token-Economy Vocabulary & Commissary Anchoring | v0.10.0 | 3/3 | Complete    | 2026-09-14 |
| 31. Lossless Token Accounting | v0.10.0 | 7/7 | Complete    | 2026-09-15 |
| 32. Unified Token Primitives | v0.10.0 | 5/5 | Complete    | 2026-09-16 |
| 33. Commissary In-Tree Adoption | v0.10.0 | 6/6 | Complete    | 2026-09-16 |
| 34. Documentation Currency Audit | v0.10.0 | 9/9 | Complete    | 2026-09-17 |
| 35. mdBook Currency | v0.10.0 | 10/10 | Complete    | 2026-09-17 |
| 36. Rustdoc Zero-Warning Bar & Examples Currency | v0.10.0 | 13/13 | Complete    | 2026-09-18 |
| 36.1. Deferred Items Closure (INSERTED) | v0.10.0 | 0/14 | Planned | — |
| 37. v0.10.0 Crate Release | v0.10.0 | 0/0 | Not started | — |

**v0.8.0 shipped 2026-08-24:** 14 phases, 149 plans, 65/65 requirements, 1,014 commits
(`be2ff05..48ac11a5`). Audit status `tech_debt` — no blockers; see
[`milestones/v0.8.0-MILESTONE-AUDIT.md`](milestones/v0.8.0-MILESTONE-AUDIT.md).

**v0.9.0 shipped 2026-09-01:** 4 phases, 25 plans, 20/20 requirements, 240 commits
(`48ac11a5..3957d701`). Audit status `tech_debt` — no blockers; see
[`milestones/v0.9.0-MILESTONE-AUDIT.md`](milestones/v0.9.0-MILESTONE-AUDIT.md). Tag `v0.9.0`
cut post-close (2026-09-01) on merge commit `0b5d4106`: release run `33542459191` fully green,
all eleven crates on crates.io at `0.9.0` — the first stable release since 0.5.1, reconciling
release numbers with milestone names. See MILESTONES.md.

## Not In This Roadmap

Deliberate omissions, so a later reader does not mistake them for oversights.

### Shipped work — the large majority of the corpus

- **Shipped Milestone-1 work.** 98% of the milestone's task items are done. The per-requirement
  record is the *Milestone 1 as-shipped ledger* in `REQUIREMENTS.md`; re-planning it as phases
  would be fiction.

- **Shipped Milestone 2-3 work — which is nearly all of it.** Sanctum and RAG (Epics 11-12),
  Sentinel vision (Epics 13, 20), autonomous planning and handoffs (Epics 14, 21), Conclave
  (Epic 15), Council and Grove (Epic 16), the Maneuver Flow DSL (Epic 17), the CLI consolidation
  and enhancement (Epics 17.5, 18), Herald consolidation (Epic 19), the Paladin registry and
  Commander metadata export (Epic 22), the scheduler port and CLI configuration wiring (Epic 23)
  and the test/benchmark hardening (Epic 24) all have shipped artefacts in the tree. Phase 5
  verifies the record; it does not rebuild the features.

- **Shipped Milestone 4-6 work — which is all of it except five defects.** The Cargo workspace and
  every crate extraction, the feature-flag matrix and CLI feature gate, and all four Milestone 6
  relocations are **verified shipped against the tree**, not merely claimed.

- **Shipped Milestone 7-8 work — which is all of it bar six verified items.** The four crate
  extractions behind the cost-benefit gate, the `Dockerfile.chef` workspace adaptation, the ten
  per-crate Makefile targets, the five-benchmark migration, the whole `v0.1.0-rc.1` release cycle,
  the 25 List A deletions, `src/core/` reduced to exactly six files, the `use_cases` → `services`
  rename, the actix removal and cargo-deny ban, the three mounted axum delivery routes, and the
  reconciliation's fifteen commits (~10,250 net LOC removed).

- **Shipped Milestone 9-12 work — which is all of it bar the record and four defects.** The whole
  Milestone 9 orchestrator subsystem (`execute_workflow()` at
  `src/application/services/orchestration/mod.rs:382`, the `WorkflowRepository` port and its SQLite
  adapter, the content processors, the orchestrator bridge, `AuthPort` and RBAC); the whole
  Milestone 10 tooling set (pre-commit with a CI gate, cargo-audit reading `audit.toml`, cargo-deny,
  OSV-Scanner with SARIF, a CycloneDX SBOM in the release pipeline, `release.toml` with
  tag-triggered publishing, the `verify-tag-source` guard and committed GitHub rulesets); the mdbook
  with `warning-policy = "error"`, mdbook-mermaid, the full chapter hierarchy and all six
  deployment-topology pages; and the whole Milestone 12 web API (agent registry and controller,
  `paladin-server`, SSE streaming, in-process jobs, the unified error envelope, health/ready,
  request logging, CORS/body-limit/timeout layers, tower-governor rate limiting, API-key and bearer
  auth with per-agent roles, OpenAPI with a committed drift baseline, `Dockerfile.server`,
  `docker-compose.yml` and `k8s/`). **37 rows verified directly against the tree.** Phase 13 records
  them; no phase rebuilds them.

### Signals that are not work

- **Open checkbox counts as a backlog.** 542 items are unchecked across 75 task lists. Five runs of
  verification found them wrong in *both* directions — understating shipped reality (Conclave 129
  and Sanctum 111, both shipped), overstating completion (CLI isolation fully checked with three
  dependencies still unconditional), contradicted outright (Milestone 8's three), vacuous
  (Milestone 12's three are feature-branch scaffolding) and nonexistent (project-management's one is
  a formatting example inside a template). **Exactly one block survives: Milestone 11's 26**, and
  DOCS-01 owns it.

- **Milestone 5's, Milestone 6's, Milestone 9's and Milestone 10's checkbox counts** — all
  corroborated or contradicted by code, none converted into tasks.

- **`REQ-master-plan-epics-11-18` as new scope.** It is the origin document for Epics 11-18, dated
  2026-01-29; every one of those epics was ingested in run 2 and most are verified shipped. Its
  value is provenance — the dependency graph and the epic-level risk assessment — not scope.

### Relocations, not gaps

- **`STABLE_API.md`, `docs/FEATURE_FLAGS.md`, `docs/MIGRATION.md`, `docs/CONFIGURATION.md`,
  `docs/PERFORMANCE_BASELINE.md`, `docs/RELEASE_CHECKLIST.md`, `docs/VERSIONING_POLICY.md`,
  `docs/BUILD_BASELINES.md`, `docs/INTEGRATION_TESTS.md`.** Absent from the paths their PRDs name,
  but shipping as mdbook chapters after the Milestone 11 overhaul — which `docs/MIGRATION_LOG.md`
  records. Recording the relocation is ARCH-05 and HARD-01; building them would be duplicate work.

- **Four stale module and document paths in run-5 requirements** — `listener_service.rs`,
  `src/application/ports/output/llm_port.rs`, `docs/Design/Design_and_Architecture.md` and the
  README demos clause. Corrected at source by ORCH-03, not rebuilt.

### Positions that would break things if implemented as written

- **The 14 requirements that shipped code superseded by outcome** (HARD-01) — actix-web as a
  `paladin-web` dependency, the `storage-sqlite` flag, the per-crate ordered publish dry run, the
  `ml` feature gate, the Milestone 8 Epic 3 no-extraction mandate, the 160-file facade target (the
  tree reads 136), and the root-path documentation deliverables.

- **A `paladin-cli` crate, MCP transport feature flags, and `vision` gating the encryption
  crates.** The last would break `cargo build --no-default-features`, because `chacha20poly1305`
  and `zeroize` serve user auth and Citadel encryption, not vision.

- **A migration between the two shipped vision surfaces.** Both ship;
  `intel/code-verification.md` records this as coexistence and says to confirm intent first.

### Explicit non-goals from the source milestones

- **Hot-reloading `config.yml`**, **terminating TLS in `paladin-server`**, **fine-grained scopes
  beyond `allowed_roles` plus the admin gate**, and **encrypting configuration at rest** — all
  Milestone 12 non-goals, recorded so they are not mistaken for omissions.

- **Rewriting the 35 mdbook appendix files** — Milestone 11 Epic 3 non-goal. One exception is under
  decision: `design-and-architecture.md`, whose relocation into that exempt chapter is precisely
  why its gap survived (DOCS-02).

- **Benchmark regression detection (`critcmp`, `github-action-benchmark`)** — Deferred-QA Epic 25
  non-goal. Note the inversion: it already ships as `benchmark-regression-signal` from Milestone 7
  Epic 3, while the `bench-check` compile prerequisite does not (PIPE-01).

- **Building `paladin-arsenal`, `paladin-sanctum` or `paladin-ml`.** None exists. The first two are
  named only by a superseded disposition record that contradicts its own governing PRD (FACADE-04);
  the third is a *placement condition* on reintroducing a removed feature (FACADE-03), not a
  deliverable.

- **A future content-delivery crate.** Reserved by Milestone 7 Epic 1 as the "correct long-term
  home" for `file_content_repository.rs`; the file was then deleted and no later document mentions
  the crate. Carried as a v2 note, not a phase.

### Decisions this roadmap records but does not take

- **Resolving the 30 competing variant groups / 69 warnings.** Recording answers is in scope
  (RECON-02 … RECON-07, VERIFY-03 … VERIFY-06, ARCH-03, ARCH-04, SEC-01, SEC-02, HARD-01 … HARD-07,
  WEB-01, PIPE-02). Picking winners inside `REQUIREMENTS.md` is not — the user has stated that
  variants are expected and that settling past disagreements is not the goal of this ingest. Where
  shipped code settles a variant, that is recorded as a **fact about the tree**, at the top of the
  precedence order, not as a decision taken here. **Group 29 is the one variant shipped code cannot
  settle**: the tree carries the Milestone 12 shape and the Milestone 9 mechanism simultaneously.

- **Promoting the eleven ADR candidates.** **Zero locked decisions exist across all 263 corpus
  documents** — no ADR-typed and no SPEC-typed document exists anywhere. Promotion requires
  re-tagging the source via `--manifest` and re-running ingest; manufacturing a lock inside a
  planning artefact would fabricate authority the corpus does not contain. SEC-01 and SUPPLY-03
  record the recommendation for the two candidates with a live operational cost — the same subject,
  from two different milestones — and do not act on it.

### Tech debt tracked as v2

- **Decomposing the three oversized service files** (2,757 / 2,294 / 1,840 lines) — real debt, no
  ingested requirement demands it.

- **Clone/lock-contention work** — the 383 `.clone()` calls and nine orchestrator locks flagged in
  `codebase/CONCERNS.md`. Blocked on Phase 3 producing benchmark evidence first.

- **The `paladin-core` / `paladin-ports` dependency allowlists** — declared 6 and 7, shipping 14 and
  10. The architectural invariant holds; this is document-versus-code drift needing ARCH-03(b) to
  choose a direction.

## Roadmap Extension Protocol

**The ingest is complete.** Five runs covered all 263 documents in `.project/` — 199 classified
(188 prose + 11 task lists) and 64 `tasks-*.md` measured deterministically. **There is no run 6.**
This section is retained because the rules below still govern any *future* addition to this
roadmap, from any source.

This roadmap is **appended to**, not restructured.

1. **Do not renumber or rewrite Phases 1-16.** Phases 1-4 are Milestone 1 close-out; 5-6 are
   Milestone 2-3; 7-8 are Milestone 4-6; 9-11 are Milestone 7-8; 12-16 are Milestone 9-12 +
   Deferred-QA. New phases start at **Phase 17** and continue upward. Use decimal insertions (e.g.
   2.1) only for urgent work that must execute *between* existing integer phases.

2. **Keep the milestone-grouped form.** Add a row to the `## Milestones` table, a labelled block
   under `## Phases`, and a new expanded `## Phase Details` section for the incoming phases. Wrap
   **only genuinely completed or superseded** milestone sections in a `<details>` block labelled
   with their milestone and status. Keep the `### Phase N: Name` header format verbatim.
   **`<details>` is a scope signal, not a rendering choice: GSD's roadmap parser strips every
   `<details>` block before phase lookup** (`stripShippedMilestones` →
   `markdown-sectionizer.stripTaggedBlocks`), so any phase wrapped in one is invisible to
   `roadmap.get-phase`, `roadmap.analyze`, and every workflow built on them — `/gsd-plan-phase`
   included. Use a plain bold label line for milestones that are not started or in progress.

3. **Add new requirement ID prefixes; do not recycle. Seventeen are spent**: `RECON-*`, `GAP-*`,
   `QUAL-*`, `REL-*` (Milestone 1); `VERIFY-*`, `CLOSE-*` (Milestone 2-3); `ARCH-*`, `DEBT-*`
   (Milestone 4-6); `SEC-*`, `HARD-*`, `FACADE-*` (Milestone 7-8); `SUPPLY-*`, `ORCH-*`, `WEB-*`,
   `PIPE-*`, `DEFER-*`, `DOCS-*` (Milestone 9-12 + Deferred-QA). Ingested `REQ-*` IDs are stable
   merge keys — match on them rather than re-deriving. **Extending an existing requirement in place
   is preferred to creating a near-duplicate**: run 4 extended ARCH-01, DEBT-01 and DEBT-03; run 5
   extended DEBT-01 again (six stale references became nine) and *corrected* SEC-01. Record the
   extension at the requirement and in the footer.

4. **Expect supersession, and record the chain.** **Zero locked decisions exist across the whole
   corpus** (0 ADR, 0 SPEC across 199 classified documents), and later milestones deliberately
   restructure earlier ones. Run 2 produced eight documented supersessions of run-1 requirements;
   run 3 produced eleven more, including the entire monolith → workspace path migration and one
   requirement a later milestone reversed outright; run 4 produced eleven more still — and the first
   case of a **document superseding another document by name**,
   `facade-cleanup-RECONCILIATION-2026-06-04.md`; run 5 produced twelve more, including the first
   case of a later run **correcting an earlier run's direct code verification**. See *Superseded but
   preserved* in `REQUIREMENTS.md`. **Relocation is not contradiction.** An ADR arriving later
   outranks anything asserted in these phases; record the supersession in `PROJECT.md` Key Decisions
   rather than silently editing a phase.

5. **Re-check the ledgers, not the phases.** If a later document claims earlier work is incomplete,
   verify against shipped code and update the relevant as-shipped ledger in `REQUIREMENTS.md`.
   Precedence for this project is **shipped tree > `.planning/codebase/` map >
   `intel/code-verification.md` > PRD > DOC > task-list checkbox.**

6. **Checkbox counts cut both ways — verify each one.** The five-run record is conclusive: counts
   understated shipped reality (runs 1-2), were accurate once and overstated once (run 3), were
   contradicted outright (run 4), and were vacuous or nonexistent (run 5). **Never convert a count
   into a requirement without checking the tree.** The trustworthy remaining-work signal in this
   corpus is the **three deferred registers** — Milestone 8's `deferred-items.md` and
   `deferred-features.md` (whose every verifiable claim matches the tree exactly, including a
   `println!` residue count exact to the occurrence), and `Deferred-QA-CICD-Completion` with
   `DEFERRED_COVERAGE.md` (whose *scope* is real and largely unbuilt, but whose *paths and numbers*
   need re-measurement) — plus the verified defects in `intel/code-verification.md`.

7. **Path claims in old PRDs are historical, including some of the newest ones.** Every
   `src/core|application|infrastructure` path in the run-1 and run-2 corpus predates the workspace
   decomposition; several run-3 paths were moved again by Milestone 6 or 8; and **four run-5
   requirements — written in June 2026 — name paths that were already gone**. Resolve current
   locations through `.planning/codebase/` or the tree, never through a PRD.

8. **Milestone numbers in source documents are not always milestone numbers.** Four instances
   exist: the M4-M6 overviews number themselves by refactoring tier, the M3 release notes assign
   Epics 19-23 to four M2 features, PRDs cross-reference "Milestone 1 / Epic 2" meaning M4 Epic 2,
   and the M7 overview titles itself "Milestone 4". In all cases the directory / task-list numbering
   is authoritative here. **A fifth was predicted in run 5 and did not occur** (ORCH-05).

9. **The Milestones 8-11 dependency graph is spent.** It described M8 → M9 **HARD**, M8 → M11
   **HARD** on path stability with M11 Epics 3-4 waiting on M9 Epics 1-3, M9 → M11 **HARD** on API
   stability, and M8 → M10 **SOFT**; critical path M8 → M9 → M11 Epics 3-5 = 11-17 sprints, M10
   entirely off it. **Run 5 confirms every dependency was honoured and every release gate was cut**
   — v0.3.0, v0.4.0, v0.5.0, v0.6.0. Keep its dependency semantics and release-gate criteria as a
   pattern; the schedule is history.

---
*Roadmap created: 2026-07-30 (ingest run 1 of 5 — `.project/Milestone_1-MVP`, 36 docs)*

*Extended: 2026-07-30 (ingest run 2 of 5 — `.project/Milestone_2-Missing_features` +
`.project/Milestone_3-Completion`, 45 docs; Phases 5-6 added, Phases 1-4 unchanged)*

*Extended: 2026-07-30 (ingest run 3 of 5 — `.project/Milestone_4-Refactor-Crates-Features` +
`.project/Milestone_5-Workspace-Decomposition` + `.project/Milestone_6-Architectural-Refinements`,
32 docs; Phases 7-8 added, Phases 1-6 unchanged. Three earlier requirements were **narrowed** by
shipped-code verification rather than renumbered — RECON-02, RECON-03 and GAP-07 — and REL-02
gained the exact edition state.)*

*Extended: 2026-07-30 (ingest run 4 of 5 — `.project/Milestone_7-Production-Hardening` +
`.project/Milestone_8-Facade-Cleanup-Shim-Resolution`, 40 docs; **Phases 9-11 added, Phases 1-8
unchanged and unrenumbered.** 16 new requirements: SEC-01 … SEC-05, HARD-01 … HARD-07,
FACADE-01 … FACADE-04. ARCH-01, DEBT-01 and DEBT-03 were **extended in place** rather than
duplicated. The Milestone 4-6 detail section was wrapped in a `<details>` block per protocol
item 2; the `### Phase N:` headers are unchanged.)*

*Extended: 2026-07-30 — **INGEST RUN 5 OF 5, FINAL. THE INGEST IS COMPLETE.**
`.project/Milestone_9-Classic-Orchestrator-Completion` +
`.project/Milestone_10-CI-Hardening-Release-Automation` +
`.project/Milestone_11-Documentation-Overhaul-Publish` + `.project/Milestone_12-Web-API` +
`.project/Deferred-QA-CICD-Completion` + `.project/project-management`, 46 docs.
**Phases 12-16 added; Phases 1-11 unchanged and unrenumbered.** 24 new requirements:
SUPPLY-01 … SUPPLY-03, ORCH-01 … ORCH-05, WEB-01 … WEB-04, PIPE-01 … PIPE-05, DEFER-01 … DEFER-03,
DOCS-01 … DOCS-04. DEBT-01 was **extended in place** a second time (six stale references became
nine) and shed its four `actions-rs` references to PIPE-04; SEC-01 was **corrected in place** —
run 4's `deny.toml`-out-of-sync finding is withdrawn, and SUPPLY-01/SUPPLY-02 carry the corrected
scope. The Milestone 7-8 detail section was wrapped in a `<details>` block per protocol item 2, and
the Overview was rewritten so this file reads as one roadmap rather than five appended fragments;
every `### Phase N:` header is unchanged and verbatim.
**Cumulative: 263 documents covered, 554 requirements, 86 forward requirements across 16 phases,
60 variant entries across 30 groups, 69 warnings, 0 blockers, 0 locked decisions, 11 ADR
candidates.***

*Corrected: 2026-07-30 (structural defect, no scope change). Runs 3, 4 and 5 wrapped the
**not-started** Milestone 1, 2-3, 4-6 and 7-8 detail sections in `<details>` blocks, citing
protocol item 2 — but item 2 reserves that wrapper for **completed or superseded** milestones, and
its claim that "downstream tooling parses it, including inside `<details>`" was false. GSD's
roadmap parser strips every `<details>` block before phase lookup, so **Phases 1-11 were invisible
to `roadmap.get-phase` and every workflow built on it**; `/gsd-plan-phase 1` failed with
`malformed_roadmap`. The four wrappers were replaced with plain bold label lines matching the
Milestone 9-12 form already used in this file, and protocol item 2 was corrected to state the
parser contract. **No phase, requirement, goal, success criterion or `### Phase N:` header was
changed** — only the four `<details>`/`<summary>`/`</details>` wrapper lines were removed. All 16
phases now resolve.*

*Extended: 2026-08-15 — **first forward addition, not ingest-derived.** Phase 17 (Additional LLM
Provider Adapters) added under a new **Provider Expansion** milestone label, per *Roadmap Extension
Protocol* item 1 ("New phases start at Phase 17 and continue upward"). Phases 1-16 unchanged and
unrenumbered; every `### Phase N:` header is verbatim. One new requirement prefix — **`PROV-*`**
(PROV-01 … PROV-04) — the eighteenth, recycling none of the seventeen spent. The phase leads with a
**provider-selection study** rather than a build list: which candidates qualify is itself the first
deliverable, and PROV-02's size is set by PROV-01's verdicts.*

*Closed: 2026-09-01 — **v0.9.0 Security Tooling shipped** (Phases 18-21, 25 plans, 20/20
requirements). Phase detail moved to `milestones/v0.9.0-ROADMAP.md` per protocol item 2; the
`<details>` wrapper above is the completed-milestone form the parser strips. Phases 1-21 are now
all shipped; the next milestone starts at Phase 22.*

*Extended: 2026-09-01 — **v0.10.0 "Durable Agent Execution Runtime" roadmap created.** Phases
22-29 added under a new milestone label, forward work sourced from the user-authored design corpus
in `.project/v0.10.0/` (program overview `00`, epic PRDs `01`-`07`, traceability matrix `08`)
rather than the historical `.project/Milestone_*` ingest — the same pattern as Phases 17-21. Eight
new requirement ID prefixes — the nineteenth through twenty-sixth, recycling none of the eighteen
spent: `ENG-*`, `CF-*`, `HITL-*`, `FT-*`, `RT-*`, `PLAT-*`, `OBS-*`, `SHIP-*` (45 requirements).
Phase order follows the program's stated dependency chain (overview §2): 22 (ENG) is the keystone;
23 (CF) depends on 22; 24 (HITL) depends on 22+23; 25 (FT) depends on 22+23 (FT-04's E2E-3 needs
CF-03/Muster); 26 (RT) is mostly standalone, depending only on 22; 27 (PLAT) depends on 22+24;
28 (OBS) depends on 22's trace seam and 27's WarGraphDoc; 29 (SHIP) is the program-gates/release
phase, depending on all of 22-28. Phases 1-21 unchanged and unrenumbered; every `### Phase N:`
header is verbatim.*

*Extended: 2026-09-14 — **v0.10.0 extended with Phases 30-33 "Token Economy"** before the
`0.10.0` tag is cut. Forward work sourced from the handoff planning corpus in
`.project/Milestone_13-Token-Economy/` (overview + Epics 1-4), authored 2026-09-14 from the
downstream Web3 Security Paladin repo's token-economy systems analysis (findings F1-F8, decisions
D-1…D-9). Four new requirement ID prefixes — the twenty-seventh through thirtieth, recycling none
of the twenty-six spent: `VOCAB-*` (7), `ACCT-*` (5), `PRIM-*` (5), `COMM-*` (4) — 21
requirements. Phase order follows the overview's dependency graph (§2): 30 (docs) can land alone
and first; 31 (accounting) is the keystone; 32 depends on 31; 33 depends on 32. **Three
scope-time amendments to the source PRDs, recorded here rather than silently applied:** (a) the
PRDs target `v0.11.0`; the operator's instruction is that this work is still part of the
**v0.10.0** release, which is possible because Phase 29 bumped `0.10.0` on the feature branch
without a tag (Phase 29 D-18/D-21) — so Phase 33 gains COMM-04, re-sealing the Phase 29 release
gates; (b) the PRDs' clean-break policy (overview §5.1: drop `Commissary::new`'s
`is_exact_counter`, remove the legacy `TokenCounter`/`TokenCounterFactory`, change the token
carriers outright, no shims) **supersedes the v0.10.0 corpus rule X-03** ("deprecations allowed,
removals are not, before v0.11.0") for Phases 31-33 only — Phase 30 gains VOCAB-07 to record that
supersession as an ADR per protocol item 4; (c) Epic 2's goals bullet ("keep a `token_count`
accessor for one release as a deprecation shim") contradicts its own R3 and §5.1 — R3 wins, no
shim (ACCT-02) — and Epic 4 R3 (wire `HistoryTrimmer` to the shared resolver) duplicates Epic 3 R4,
so it lives once, in Phase 32 (PRIM-04), with Phase 33 keeping only the regression check.
**Treasurer (Milestone 14, `.project/Milestone_14-Treasurer/`) is reserved by Phase 30's ADR and
not roadmapped** — operator-confirmed deferral; it depends on Phase 31 and is the natural first
phase of the next milestone. Phases 1-29 unchanged and unrenumbered; every `### Phase N:` header
is verbatim.*

*Extended: 2026-09-17 — **v0.10.0 extended with Phases 34-37 "Release Readiness"** before the
`0.10.0` tag is cut. Operator-instructed, not corpus-sourced: with all 13 phases verified, the
pre-tag review found the mdBook not updated for the milestone's changes, the rustdoc corpus failing
CI's zero-tolerance "Check documentation" step (73 carried `cargo doc` warnings per
`33-CI-EVIDENCE.md` row 26; 14 unresolved intra-doc links under `--all-features`, `WINDOWS.md`
#36), and the examples likely stale against the Phase 22-33 API — with the gap possibly reaching
back into v0.9.0. Phase order: 34 audits first (read-only, producing the inventory that scopes 35
and 36); 35 (mdBook) and 36 (rustdoc + examples) are independent siblings that both depend on 34
and may run in parallel; 37 (release) depends on both and repeats the Phase 33 gate re-seal on the
final commit before the merge, the tag and the crates.io publish. Requirement IDs are deferred to
planning — one new prefix will be needed (the thirty-first; `DOCS-*` is spent) — and Phase 37 may
extend SHIP-04 in place per protocol item 3. Phases 1-33 unchanged and unrenumbered; every
`### Phase N:` header is verbatim.*
