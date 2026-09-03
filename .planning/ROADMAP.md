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
| **Durable Agent Execution Runtime** | 22-29 | 🔲 Not started | Forward work — not ingest-derived. Added 2026-09-01, sourced from the user-authored design corpus in `.project/v0.10.0/` (program overview `00`, epic PRDs `01`-`07`, traceability matrix `08`) rather than the historical `.project/Milestone_*` ingest. |

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

**Durable Agent Execution Runtime (Phases 22-29)** — not started (added 2026-09-01)

- [x] **Phase 22: Battlefield State & Superstep Engine** - Typed shared state, cyclic superstep execution, and automatic per-superstep checkpointing that resumes with zero re-execution after a crash (completed 2026-09-02)
- [ ] **Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs** - Directive-based routing, Muster dynamic fan-out, nested Battalion subgraphs, LLM-evaluated routing, and the BUG-01 fail-closed fix
- [ ] **Phase 24: Pause/Resume, History & Graceful Shutdown** - Indefinite Parley pauses, typed resume validation, an inspectable/forkable Chronicle, graceful shutdown, and Thread endpoints over HTTP
- [ ] **Phase 25: Node-Level Fault Tolerance** - Typed error transience, per-node Aegis retry, wall/idle timeouts, typed compensation handlers, provider fallback, and node result caching
- [ ] **Phase 26: Agent Runtime Enhancements** - Execution middleware chain, context-window management, cross-session Vault memory, structured output, provider conformance close-out, and a one-line reasoning agent
- [ ] **Phase 27: Platform API** - Durable background runs on a worker pool, Parley/streaming integration, versioned assistants, and API-managed schedules/webhooks
- [ ] **Phase 28: Observability & Tooling** - Machine-consumable trace stream, OTel/log/SSE consumers, graph/run visualization, and the paladin-eval regression harness
- [ ] **Phase 29: Program Gates & Release** - Complete MIGRATION.md, proven backward compatibility, the program acceptance audit, and a releasable v0.10.0

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
milestone. Phases 22-29 — the current milestone, v0.10.0 "Durable Agent Execution
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
**Plans:** 5 plans

Plans:
**Wave 1**

- [x] 22.1-01-PLAN.md — BUG-03 cycle-bootstrap starvation fix (tracer, test-first) + BUG-03/ENG-FR-06a registration
- [x] 22.1-04-PLAN.md — MSRV floor to measured 1.88, lockfile restore, resolver 3, MSRV figure reconciliation

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 22.1-02-PLAN.md — Truthful-outcome guards: validate-time unschedulable-shape check, run-end starvation check, determinism coverage

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 22.1-03-PLAN.md — CR-01 fingerprint coverage + golden/difference tests + resume-doc reword + fixture comment sweep

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 22.1-05-PLAN.md — G-22-1 closing CI evidence (whole-run success) + UAT pointer

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

**Plans**: TBD

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

**Plans**: TBD

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

**Plans**: TBD

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

**Plans**: TBD

### Phase 27: Platform API

**Goal**: Runs execute durably in the background on a worker pool, integrate with Parley pauses and live streaming, and are managed through versioned assistants, cron schedules and webhooks — all reachable over a production-shaped HTTP API.
**Depends on**: Phase 22, Phase 24
**Requirements**: PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06
**Success Criteria** (what must be TRUE):

  1. `POST /runs` returns 202 within 250ms p99 (enqueue only), a `RunRepositoryPort` persists every status transition, and the status machine is monotonic with typed illegal-transition errors (PLAT-01)
  2. A worker pool executes runs via `RunQueuePort` (InMemory/Redis) with lease heartbeats, at-least-once redelivery that resumes a kill-mid-run thread rather than restarting it, cross-instance cancellation observed at superstep boundaries, and a `409 ThreadBusy` invariant holding under 10 concurrent submits (PLAT-02)
  3. `AwaitingInput` releases the worker, `POST /threads/{id}/resume` re-enqueues under the same `run_id`, and `GET /runs/{id}/stream` bridges live TraceSink events to SSE with a documented polling-backed degraded mode and 15s heartbeats (PLAT-03)
  4. Assistants are append-only immutable versions (no PUT, ever) with `latest` frozen at submit time, and `WarGraphDoc` compiles through a registry-resolving `compile()` with a restart-stable fingerprint round-trip (PLAT-04)
  5. Cron schedules survive restart without duplicate or missed-then-double firing; HMAC-signed webhook delivery retries bounded on 5xx/timeout with an SSRF guard rejecting non-http(s)/loopback/link-local/private/metadata targets; and every new endpoint carries existing auth, rate limiting, scopes and pagination, with `openapi.json` regenerated and Python/TypeScript clients generated and smoke-tested in CI (PLAT-05, PLAT-06)

**Plans**: TBD

### Phase 28: Observability & Tooling

**Goal**: Every run emits a machine-consumable trace that reaches real consumers, graphs and runs are visualizable, and agent behavior is regression-testable.
**Depends on**: Phase 22 (trace seam), Phase 27 (WarGraphDoc)
**Requirements**: OBS-01, OBS-02, OBS-03, OBS-04
**Success Criteria** (what must be TRUE):

  1. The authoritative `TraceEvent` enum carries a per-run monotonic `seq` with a gapless-or-counted-drops guarantee, and a `TraceSinkPort` whose slow or panicking implementations cannot stall or fail a run fans out via `CompositeSink` (OBS-01)
  2. Traces reach a default-on structured-log sink, an `otel`-gated OpenTelemetry exporter with span-per-attempt trees verified against a collector stub, and the SSE bridge for `GET /runs/{id}/stream` as a TraceSink adapter (OBS-02)
  3. Golden-tested `WarGraphDoc → Mermaid/DOT` exporters and an execution-overlay export let a human answer "which branch fired and why did node X run 3 times" via `paladin-cli graph export`/`run export` and a minimal auth-gated `dev-ui` inspector page (OBS-03)
  4. The new `paladin-eval` crate runs scripted mock-LLM scenario files through a `cargo test`-integrable runner macro and `paladin-cli eval run --repeat`/`--bless`, with the three program E2E fixtures dogfooded as eval scenarios (OBS-04)

**Plans**: TBD
**UI hint**: yes

### Phase 29: Program Gates & Release

**Goal**: v0.10.0 is releasable — the migration record is complete, backward compatibility is proven rather than asserted, the program acceptance audit passes, and every crate publishes.
**Depends on**: Phase 22, Phase 23, Phase 24, Phase 25, Phase 26, Phase 27, Phase 28 (all)
**Requirements**: SHIP-01, SHIP-02, SHIP-03, SHIP-04
**Success Criteria** (what must be TRUE):

  1. `MIGRATION.md` has every §9 section filled with no "TBD" — M-B-01…03 resolved with chosen defaults and worked examples, the §9.2 register matching the `cargo semver-checks` allowlist exactly — and is linked from the README and the mdBook "Upgrading" page (SHIP-01)
  2. An integration test boots v0.10 with a v0.9 sample config and asserts legacy behavior (all new subsystems disabled by default), and a golden diff of `openapi.json` restricted to pre-existing paths is empty (SHIP-02)
  3. E2E-1/2/3 pass green as integration tests in `tests/`, the doc-08 verification protocol confirms every FR has a passing test with no orphan behavior and ubiquitous-language names conform, and BUG-01's old warn-and-default-true path is grep-absent with the fix's failing-then-passing test order visible in history (SHIP-03)
  4. All workspace crates are at `0.10.0` with changelogs updated, `cargo publish --dry-run` is green for every publishable crate in dependency order, mdBook + rustdoc are updated with no new broken intra-doc links, and the semver and MSRV CI jobs are green on the release commit (SHIP-04)

**Plans**: TBD

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
| 23. Control Flow — Dynamic Routing, Fan-Out & Subgraphs | v0.10.0 | 0/0 | Not started | - |
| 24. Pause/Resume, History & Graceful Shutdown | v0.10.0 | 0/0 | Not started | - |
| 25. Node-Level Fault Tolerance | v0.10.0 | 0/0 | Not started | - |
| 26. Agent Runtime Enhancements | v0.10.0 | 0/0 | Not started | - |
| 27. Platform API | v0.10.0 | 0/0 | Not started | - |
| 28. Observability & Tooling | v0.10.0 | 0/0 | Not started | - |
| 29. Program Gates & Release | v0.10.0 | 0/0 | Not started | - |

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
