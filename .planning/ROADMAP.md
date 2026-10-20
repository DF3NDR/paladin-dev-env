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
| **Durable Agent Execution Runtime** | 22-37.1 | ✅ **Shipped v0.10.0 (2026-09-23)** — [archive](milestones/v0.10.0-ROADMAP.md). Released to crates.io as `0.10.1` (tag `v0.10.1` on merge commit `f7dae267`, 2026-09-21) after the `v0.10.0` tag published 3 of 12 crates — see MILESTONES.md. Phases 22-29 complete 2026-09-10; Phases 30-33 added 2026-09-14, complete 2026-09-16; Phases 34-37 added 2026-09-17, Phase 36.1 and 37.1 inserted; all 19 phases complete 2026-09-23 | Forward work — not ingest-derived. Added 2026-09-01, sourced from the user-authored design corpus in `.project/v0.10.0/` (program overview `00`, epic PRDs `01`-`07`, traceability matrix `08`) rather than the historical `.project/Milestone_*` ingest. **Extended 2026-09-14** with Phases 30-33 (Token Economy — Commissary anchoring, lossless accounting, primitive unification, Commissary adoption), sourced from `.project/Milestone_13-Token-Economy/` (overview + Epics 1-4). Milestone 14 (Treasurer, `.project/Milestone_14-Treasurer/`) is reserved, not roadmapped. **Extended 2026-09-17** with Phases 34-37 (Release Readiness — documentation currency audit, mdBook currency, rustdoc zero-warning bar & examples currency, the crate release) — operator-instructed pre-tag work, not corpus-sourced. |
| **Treasurer Spend Governance** | 38-50 | 🚧 **In progress** | Forward work — not ingest-derived. Sourced from `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD `prd-treasurer-spend-governance.md`, R1-R6), the supporting scope in PROJECT.md *Current Milestone*, and `.planning/research/SUMMARY.md` (2026-09-24). Operator-confirmed full governance (per-tenant + per-API-key), 2026-09-24. Extended 2026-10-12 with Phase 48 (Base Entity Rename) and 2026-10-14 with Phase 49 (Downstream Adoption of v0.10.1, from the web3sec handoff), both sequenced before Phases 46-47; extended again 2026-10-14 with Phase 50 (Provider Parity & Configuration-Driven Model Swap, from the second web3sec handoff), also sequenced before Phases 46-47; extended again 2026-10-19 with Phase 50.1 (Consumer-Surface Readiness for v0.11.0, inserted from the fourth web3sec handoff), sequenced after Phase 50 and before Phase 51. |

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

<details>
<summary>✅ <strong>v0.10.0 Durable Agent Execution Runtime (Phases 22-37.1)</strong> — SHIPPED 2026-09-23 · 231 plans (228 executed, 3 superseded), 88/89 requirements (SHIP-05 superseded by SHIP-06) · released to crates.io as <code>0.10.1</code></summary>

- [x] **Phase 22: Battlefield State & Superstep Engine** - Typed shared state, cyclic superstep execution, and automatic per-superstep checkpointing that resumes with zero re-execution after a crash (completed 2026-09-02)
- [x] **Phase 22.1: Engine readiness defect and MSRV follow-up (INSERTED)** - Fix the BUG-03 cycle-bootstrap starvation and BUG-04 resume-frontier defects, complete the graph fingerprint, raise the MSRV floor to a measured 1.88, and seal G-22-1 on whole-run CI evidence (completed 2026-09-03)
- [x] **Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs** - Directive-based routing, Muster dynamic fan-out, nested Battalion subgraphs, LLM-evaluated routing, and the BUG-01 fail-closed fix (completed 2026-09-04)
- [x] **Phase 24: Pause/Resume, History & Graceful Shutdown** - Indefinite Parley pauses, typed resume validation, an inspectable/forkable Chronicle, graceful shutdown, and Thread endpoints over HTTP (completed 2026-09-05)
- [x] **Phase 25: Node-Level Fault Tolerance** - Typed error transience, per-node Aegis retry, wall/idle timeouts, typed compensation handlers, provider fallback, and node result caching (completed 2026-09-06)
- [x] **Phase 26: Agent Runtime Enhancements** - Execution middleware chain, context-window management, cross-session Vault memory, structured output, provider conformance close-out, and a one-line reasoning agent (completed 2026-09-07)
- [x] **Phase 27: Platform API** - Durable background runs on a worker pool, Parley/streaming integration, versioned assistants, and API-managed schedules/webhooks (completed 2026-09-08)
- [x] **Phase 28: Observability & Tooling** - Machine-consumable trace stream, OTel/log/SSE consumers, graph/run visualization, and the paladin-eval regression harness (completed 2026-09-09)
- [x] **Phase 29: Program Gates & Release** - Complete MIGRATION.md, proven backward compatibility, the program acceptance audit, and a releasable v0.10.0 (completed 2026-09-10)
- [x] **Phase 30: Token-Economy Vocabulary & Commissary Anchoring** - Record the units-plain / roles-medieval vocabulary rule, anchor `Commissary` with an ADR and an mdBook page, reserve `Treasurer` with its downstream guardrail, document the four `max_tokens` meanings, purge the orphan `Quartermaster` references, and record the clean-break versioning decision as an ADR (docs only) (completed 2026-09-14)
- [x] **Phase 31: Lossless Token Accounting** - Carry the full `TokenUsage` prompt/completion split (plus optional cache/reasoning fields) from the LLM port to `RunFinished` and a herald, remove the `from_total` zeroing from the battalion path, and prove streaming usage parity per adapter (keystone; breaking) (completed 2026-09-15)
- [x] **Phase 32: Unified Token Primitives** - One counting contract (`TokenCounterPort::is_exact`, `Commissary::new` drops `is_exact_counter`, legacy `TokenCounter`/`TokenCounterFactory` retired) and one shared context-window resolver with a strict mode consumed by both `HistoryTrimmer` and `Commissary` (breaking) (completed 2026-09-16)
- [x] **Phase 33: Commissary In-Tree Adoption** - Route RAG truncation through `Commissary::dispense` with shed records and a truncation marker, closing the last silent-truncation path with an integration-tested production caller, then re-seal the Phase 29 release gates on the final commit (completed 2026-09-16)
- [x] **Phase 34: Documentation Currency Audit** - Audit the mdBook, the rustdoc corpus and the `examples/` / `doc-examples` programs against everything Phases 22-33 changed (and anything v0.9.0 left unwritten), producing one classified gap inventory that scopes Phases 35-36 (read-only; no docs change) (completed 2026-09-17)
- [x] **Phase 35: mdBook Currency** - Close every mdBook gap the Phase 34 inventory records so the book describes the v0.10.0 tree — new pages where a capability shipped without one, corrected pages where the API or vocabulary changed, and `mdbook build` + linkcheck green (completed 2026-09-17)
- [x] **Phase 36: Rustdoc Zero-Warning Bar & Examples Currency** - Take `cargo doc --workspace --no-deps` from 73 carried warnings to zero so CI's "Check documentation" step is green, resolve the 14 `--all-features` intra-doc links, and bring every `examples/` and `doc-examples` program current with the Phase 22-33 API (completed 2026-09-18)
- [x] **Phase 36.1: Deferred Items Closure (INSERTED)** - Every deferred item Phases 30-35 recorded and left unowned is closed in the tree or explicitly dispositioned before v0.10.0 ships — the Phase 31/32/34/35/36 registers, ledger rows 36-38 and the two pending todos are walked item by item, and `WINDOWS.md` agrees with the registers (`open_count: 0`) (completed 2026-09-18)
- [x] **Phase 37: v0.10.0 Crate Release** - Re-seal the Phase 29 release gates on the post-documentation final commit, merge to `main`, cut the `v0.10.0` tag through `release.yml`, and confirm every publishable crate is on crates.io at `0.10.0` (completed 2026-09-22)
- [x] **Phase 37.1: v0.10.1 Patch Release (INSERTED)** - Correct the partial `v0.10.0` publish with a `0.10.1` patch release through the same pipeline: fix the two defects that broke `v0.10.0` (`paladin-battalion`'s forward-pointing versioned dev-dependencies and the `create-or-reuse-release.sh` `EPIPE` race), add a per-crate publish-order gate that fails on the `1d4a9724` tree, and confirm every publishable crate is on crates.io at `0.10.1` (completed 2026-09-23)

Full detail: [`milestones/v0.10.0-ROADMAP.md`](milestones/v0.10.0-ROADMAP.md) ·
Audit: [`milestones/v0.10.0-MILESTONE-AUDIT.md`](milestones/v0.10.0-MILESTONE-AUDIT.md) ·
Requirements: [`milestones/v0.10.0-REQUIREMENTS.md`](milestones/v0.10.0-REQUIREMENTS.md) ·
Phase artifacts: `milestones/v0.10.0-phases/`

</details>

**Treasurer Spend Governance (Phases 38-51)** — in progress, not yet shipped. Forward work sourced
from `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD R1-R6) and
`.planning/research/SUMMARY.md`; requirements minted in `.planning/REQUIREMENTS.md`.

- [x] **Phase 38: Design Seams & Pricing/Cost Producer** - Record the mid-run enforcement attachment point and the ledger balance model, then ship an operator-configured price table and a fixed-point cost function that reaches `ExecutionMetadata.cost_estimate` (completed 2026-09-26)
- [x] **Phase 39: Spend Ledger** - A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters behind one contract suite, reserve-then-settle draws, idempotent settlement, and queryable spend (completed 2026-09-28)
- [x] **Phase 40: Tenant Identity & Run-Read Scoping** - Map API keys to tenants on the `Principal`, attribute every run to its submitting principal, and scope every run-read route to the calling principal (completed 2026-09-30)
- [x] **Phase 41: Admission-Time Allowance Enforcement** - Rolling-period allowances with an optional lifetime cap, refused at admission when exhausted, with a non-blocking warn-threshold notice (completed 2026-10-12)
- [x] **Phase 42: Mid-Run Halt & SSE Terminal Status** - A resumable, checkpoint-preserving halt when an in-flight run's next draw would overspend, on both the engine and agent-loop paths, plus the SSE `done`/`Cancelled` fix (completed 2026-10-07)
- [x] **Phase 43: Rate Pacing** - In-process and Redis-shared 429 back-off with jitter, a distributed cache-stampede lock, and safe degradation when Redis is unavailable (completed 2026-10-09)
- [x] **Phase 44: Legacy Clean-Break Removal** - Remove legacy Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts and `PaladinError::LlmError(String)`, migrating the last string-matching retry check to the typed taxonomy (completed 2026-10-11)
- [x] **Phase 45: RustFS Swap & Platform/Observability Deviations** - Replace MinIO with RustFS across dev/test and CI, wire SSE/webhook emission for legacy `Runnable::Agent` runs, and close the tracing-overhead gap (completed 2026-09-30)
- [ ] **Phase 46: Docs Currency & Hygiene** - Close the v0.10.0 audit's docs fixes, publish the Treasurer mdBook page and migration guide, reconcile Nyquist validation, and dispose of the three v2 debt lines
- [ ] **Phase 47: v0.11.0 Crate Release** - Tag and publish v0.11.0: all 12 crates on crates.io, CHANGELOG and MIGRATION complete, publish-order gate green
- [x] **Phase 48: Base Entity Rename** - Rename the base-layer entity wrapper `Node<T>` → `Entity<T>` (payload field `node` → `data`; clean break under a Phase-48-only X-03 supersession, or a one-release deprecated alias — decided at discuss; serde alias for persisted JSON), disposition the unused base-layer items, and reserve "node" for the graph sense only — **executes before Phases 46 and 47** (added 2026-10-12, after Phase 44 sealed) (completed 2026-10-14)
- [x] **Phase 49: Downstream Adoption of v0.10.1 (web3sec Consumer Bump)** - Scope cut to its Paladin-side half on 2026-10-14 (owner decision): the compile-only consumer-surface muster and the `MIGRATION.md` library-consumer note landed (plans 49-01..03); the consumer bump itself (ADOPT-01..03) is superseded by one web3sec move straight to `v0.11.0` after Phases 50 and 51 (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`) (completed 2026-10-14)
- [x] **Phase 50: Provider Parity & Configuration-Driven Model Swap** - Make any Paladin agent movable between the nine LLM providers by changing configuration only, or fail the build early naming the provider, model and parameter: fix the OpenAI adapter's empty user message, model parameter support per provider *and* per model (ADR amending 0004), config-driven provider construction through a public library API with all nine adapter/config pairs exported from the facade, a dated per-model profile registry feeding Commissary, response/error parity (empty output, typed refusal, usage cap, cache-write, reasoning text, stop sequences), per-step models in fallback chains, and all nine adapters in the conformance suite, live smoke and an mdBook "Switching providers" page — **executes after Phase 49, before Phases 46 and 47** (added 2026-10-14 from the second web3sec handoff) (completed 2026-10-19)
- [x] **Phase 50.1: Consumer-Surface Readiness for v0.11.0 (INSERTED)** - Close the six consumer-surface gaps web3sec's pre-measured bump found before the tag: the Aegis family (`Aegis`, `ErrorHandlerSpec`, `TimeoutPolicy`, `StateDelta`, `NodeError`, `NodeErrorSource`, `TimeoutKind`, `NodeId`, `Transience`) nameable through the `paladin` facade so MIGRATION §9.8 DR-1/2/8/9 compile without `paladin-ai-core`; one `new(..)` shape across the nine `*Config` types plus `with_timeout_seconds` (and `with_base_url`) on the shared knob builders as a registered clean break; the consumer-surface muster extended to everything the consumer actually uses with a red proof; four §9.8 gaps closed (pre-`v0.10.0` P31/P32/P33 pointers, the loss of in-adapter 429 retry and the Cadence recipe, decorator `model_profile` forwarding, the adapter-wide context window following the seeded default model); a facade-reachable pacing-only composition path; and seeds for `claude-opus-4-8` / `deepseek-v4-pro` under the seed rule — **executes after Phase 50, before Phase 51** (inserted 2026-10-19 from the fourth web3sec handoff; owner decision 2026-10-18) (completed 2026-10-20)
- [ ] **Phase 51: Adjustable Reasoning Effort** - One provider-neutral `ReasoningEffort` setting per request, per agent and per provider that every adapter translates exactly to its vendor's native wire form (compile-enforced per compat preset, model-aware in the native adapters, levels read from Phase 50's model-profile registry) or refuses loudly before the HTTP call with a new additive `LlmError::UnsupportedParameter`; a silent-no-op regression test for Kimi, requested and effective effort on the response and trace stream, `Budget(n)` validated against vendor bounds and `max_tokens`, live per-vendor proof that the effort field is parsed, an ADR amending 0004 and an mdBook "Reasoning effort" page — **executes after Phase 50.1, before Phases 46 and 47** (added 2026-10-14 from the third web3sec handoff; resequenced behind the inserted Phase 50.1 on 2026-10-19 so it plans against the harmonised `*Config::new` shape; the handoff read `v0.10.1`, the feature ships in v0.11.0)

## Phase Details

*Phases 1-4 are archived in [`milestones/v0.7.1-ROADMAP.md`](milestones/v0.7.1-ROADMAP.md).
Phases 5-17 — every phase of the v0.8.0 milestone, with their full goals, success criteria,
amendment banners and per-plan checklists — are archived in
[`milestones/v0.8.0-ROADMAP.md`](milestones/v0.8.0-ROADMAP.md), together with
[`v0.8.0-REQUIREMENTS.md`](milestones/v0.8.0-REQUIREMENTS.md) and
[`v0.8.0-MILESTONE-AUDIT.md`](milestones/v0.8.0-MILESTONE-AUDIT.md). Phases 18-21 — the v0.9.0
Security Tooling milestone — are archived in
[`milestones/v0.9.0-ROADMAP.md`](milestones/v0.9.0-ROADMAP.md), together with
[`v0.9.0-REQUIREMENTS.md`](milestones/v0.9.0-REQUIREMENTS.md) and
[`v0.9.0-MILESTONE-AUDIT.md`](milestones/v0.9.0-MILESTONE-AUDIT.md). Phases 22-37.1 — the
v0.10.0 "Durable Agent Execution Runtime" milestone, including the Token Economy and Release
Readiness extensions and the three inserted phases — are archived in
[`milestones/v0.10.0-ROADMAP.md`](milestones/v0.10.0-ROADMAP.md), together with
[`v0.10.0-REQUIREMENTS.md`](milestones/v0.10.0-REQUIREMENTS.md) and
[`v0.10.0-MILESTONE-AUDIT.md`](milestones/v0.10.0-MILESTONE-AUDIT.md). Only phases in the current
and future milestones are detailed below, which is what keeps this file a constant size per
milestone. Phases 38-47 (v0.11.0 Treasurer Spend Governance) are detailed below.*

### Phase 38: Design Seams & Pricing/Cost Producer

**Goal**: An operator can price every LLM call in a configured currency and see an honest per-run
cost, with the two gating architecture decisions — the mid-run Treasurer enforcement attachment
point across `WarEngine` and `PaladinExecutionService`, and derive-on-read vs a running-balance
ledger model — recorded before any dependent phase begins.
**Depends on**: Nothing (first phase of v0.11.0)
**Requirements**: PRICE-01, PRICE-02, PRICE-03
**Success Criteria** (what must be TRUE):

  1. Operator can configure a per-model price table (prompt, completion, cache-read, cache-write,
     reasoning) as decimal strings; the default table is empty, and a negative or malformed price
     is rejected at config validation, not accepted silently.

  2. For any `TokenUsage`, cost is computed as an exact fixed-point integer micro-unit value with no
     `f64` accumulation, unit-tested per token type including cache and reasoning tokens.
     *(Amended 2026-09-25, Phase 38 plan 38-01: the cost unit is i64 nano-units (1e-9), a finer
     scale that satisfies "micro-units" — see 38-CONTEXT.md D-02 and ADR-0053.)*

  3. A completed run's `ExecutionMetadata.cost_estimate` carries the currency cost end-to-end when
     its model has a configured price, and is `None` — never `0` — when it does not; the field's
     reserved rustdoc note now reads "produced by the Treasurer".

  4. Two ADRs are on record — the mid-run enforcement attachment point and the ledger balance
     model — and the Phase 39/41/42 plans reference them rather than re-opening the question.
**Plans**: 9 plans (6 waves)

Plans:

**Wave 1**

- [x] 38-01-PLAN.md — ADR-0052 (mid-run Treasurer enforcement attachment point) and ADR-0053 (append-only, derive-on-read ledger; row kinds, i64 nano-unit amounts, settlement key) with the PROMOTION.md index, one blocking checkpoint on settlement granularity (D-13..D-16)

**Wave 2** *(blocked on Wave 1)*

- [x] 38-02-PLAN.md — tracer: a priced streamed agent call reaches `ExecutionMetadata` and the markdown herald (`0.0225 USD`); `Cost`/`PriceTable`/`CostTally` arithmetic pinned per axis (PRICE-02, PRICE-03)

**Wave 3** *(blocked on Wave 2)*

- [x] 38-03-PLAN.md — the `treasurer:` config section (exact decimal parsing, boot validation) and pricing installed on both production run paths (PRICE-01)
- [x] 38-04-PLAN.md — `LlmResponse.cost` with every literal migrated and the break registered; non-streaming and fallback-hop pricing (PRICE-03)
- [x] 38-05-PLAN.md — JSON herald currency field; table herald renders real metadata and cost (PRICE-03)
- [x] 38-06-PLAN.md — `NodeFinished.cost`/`RunFinished.cost` carriers and `TraceDispatcher::total_cost` (PRICE-03)

**Wave 4** *(blocked on Wave 3)*

- [x] 38-07-PLAN.md — `PaladinResult.cost`, agent-loop cost accumulation, and the engine bridge from each Paladin attempt to `NodeFinished.cost` (PRICE-03)

**Wave 5** *(blocked on Wave 4)*

- [x] 38-08-PLAN.md — engine-path `ExecutionMetadata` producer: `from_run_finished`, `HeraldTraceSink`, `RunWorkerPool::with_herald` (PRICE-03)

**Wave 6** *(blocked on Wave 5)*

- [x] 38-09-PLAN.md — closeout: `cargo semver-checks` measurement and registration, CHANGELOG, API-surface refresh, full gate suite (PRICE-01..03)

**Research flag**: yes — the engine-path-vs-agent-loop attachment point for mid-run `TokenBudget`
enforcement is, per research, the single largest open architectural question this milestone
surfaces; needs an ADR-level design pass before Phase 42 planning.

### Phase 39: Spend Ledger

**Goal**: A durable, race-proof spend ledger exists that every later Treasurer phase can draw
against and query.
**Depends on**: Phase 38 (cost function to persist; reserve/settle schema decision)
**Requirements**: LEDGR-01, LEDGR-02, LEDGR-03, LEDGR-04
**UI hint**: no
**Success Criteria** (what must be TRUE):

  1. One shared `TreasuryLedgerPort` contract-test suite passes unmodified against in-memory,
     SQLite and Postgres adapters, backed by a `007` migration in both
     `crates/paladin-storage/migrations/{sqlite,postgres}/`.

  2. When N concurrent draws race against a balance that only N−1 of them fit, exactly N−1 succeed
     and one is refused, on every adapter.

  3. Settling the same run/superstep/attempt twice — via lease redelivery, resume, retry or model
     fallback — charges it exactly once.

  4. Operator can view spend per tenant, API key, run and model over a time window from the CLI,
     and that spend appears in herald output and trace events.
**Plans**: 8 plans (5 waves)

Plans:

**Wave 1**

- [x] 39-01-PLAN.md — blocking checkpoint on the one-way `007` schema (D-01/D-06), then the tracer: `TreasuryLedgerPort` and core ledger types, the SQLite `007` migration and `SqliteTreasuryLedger` settle/spend, `paladin-cli treasury spend` printing `0.0450 USD` (LEDGR-01, LEDGR-04)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 39-02-PLAN.md — race-proof `reserve`/`release` and idempotent `settle` on SQLite (`BEGIN IMMEDIATE`, partial unique settlement index), the shared contract suite opened by the N−1-of-N race clause, `InMemoryTreasuryLedger` (LEDGR-01, LEDGR-02, LEDGR-03)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 39-03-PLAN.md — Postgres `007` migration and `PostgresTreasuryLedger` serialized per scope with a transaction-scoped advisory lock, passing the contract suite unmodified; CLI Postgres backend (LEDGR-01, LEDGR-02, LEDGR-03)
- [x] 39-04-PLAN.md — engine-path settle writer: one settlement per superstep attempt at the ADR-0052 boundary, aggregating priced Paladin attempts with a per-model breakdown; `WarEngine::with_treasury_ledger` (LEDGR-03, LEDGR-04)
- [x] 39-05-PLAN.md — agent-loop settle writer: one settlement per priced model call in `PaladinExecutionService`, `RunScope::with_run_id`, an `AgentLoopSettlement` mode that never double-counts engine nodes (LEDGR-03, LEDGR-04)
- [x] 39-06-PLAN.md — ledger-derived `cost` on `GET /runs` and `GET /runs/{id}` and `PaladinResult.cost` on the agent execute response through a shared `CostDto`; inverts `execute_response_carries_no_cost_field` (LEDGR-04)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 39-07-PLAN.md — production wiring: the worker hands each engine the ledger and the persisted `runs.attempt`, agent-kind runs carry their Platform run id, the server builds the ledger from `RunStoreConfig`; HTTP end-to-end proof (LEDGR-03, LEDGR-04)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 39-08-PLAN.md — closeout: `cargo semver-checks` measurement and registration, CHANGELOG entries, facade re-export, API-surface refresh, full gate suite and credential-handling review (LEDGR-01..04)

### Phase 40: Tenant Identity & Run-Read Scoping

**Goal**: Every run is attributable to the tenant and API key that submitted it, and no caller can
read another caller's runs.
**Depends on**: Phase 38 (shares the milestone's config/identity conventions); independent of
Phase 39
**Requirements**: TENANT-01, TENANT-02, PLAT-07
**Success Criteria** (what must be TRUE):

  1. Every API key in operator config maps to a tenant, the authenticated `Principal` carries
     `tenant_id`, and a caller cannot assert a different tenant than its own key's mapping.

  2. Every submitted run records its submitting principal (API key id and tenant).
  3. `GET /runs` and every `/runs/{id}*` read route return only runs the calling principal may
     see — another caller's run returns 404 — enforced by one shared authorization function used
     on every such route, not duplicated per endpoint.

  4. The breaking `Principal` change is recorded in `MIGRATION.md` §9.2 and the
     `cargo semver-checks` allowlist.
**Plans**: 6 plans

Plans:
**Wave 1**

- [x] 40-01-PLAN.md — Checkpoint on the one-way doors (D-01/D-04/D-09), then the tracer: API-key tenant → Principal → PrincipalRef → Run.submitted_by → SQLite 008 → GET /v1/runs/{id} gate (owner/Admin 200, other tenant 404); register the breaks (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 40-02-PLAN.md — Scoped GET /v1/runs: RunQuery.scope inside SQL on all three adapters, Postgres attribution parity, shared contract clauses (wave 2)
- [x] 40-03-PLAN.md — AuthConfig::validate fail-closed key/tenant/bearer rules at boot and deployment docs (wave 2)
- [x] 40-04-PLAN.md — Ledger scope source: LedgerScope::from_attribution on the worker path and scoped executor ports on the HTTP agent path (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 40-05-PLAN.md — load_visible_run on stream/cancel/webhook-deliveries, route matrix, RunResponse.submitted_by, read-scope docs and MIGRATION.md 9.6 (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 40-06-PLAN.md — Release records: semver measurement and register reconciliation, CHANGELOGs, surface baseline, ADR-0054, WINDOWS.md, full gates (wave 4)

### Phase 41: Admission-Time Allowance Enforcement

**Goal**: A tenant or API key that has exhausted its allowance is stopped before a run is ever
persisted, and the operator gets an early warning before that happens.
**Depends on**: Phase 39 (ledger to evaluate spend against), Phase 40 (identity carrier)
**Requirements**: ALLOW-01, ALLOW-02, ALLOW-04
**Success Criteria** (what must be TRUE):

  1. Operator can configure a rolling-period allowance (with an optional lifetime cap) per tenant
     and per API key, distinct from every existing `max_tokens` meaning; window boundaries are
     computed from the store or server clock, never a worker's local clock.

  2. Submitting a run while the caller's tenant or API-key allowance is exhausted is refused with a
     typed error before any run row is written.

  3. Crossing a configurable warn threshold (e.g. 80%) emits exactly one trace event plus a herald
     and webhook notice per window, without blocking the run.
**Plans**: 9 plans

Plans:
**Wave 1**

- [x] 41-01-PLAN.md — Checkpoint on the consolidated design (C1/C3/C5/C6/C10/C11/C13 against D-04/D-08/D-16/D-17), then the tracer: per-API-key window allowance → AllowancePolicy → Treasurer → store_now + TreasuryLedgerPort::balance → submit refuses before any row → 429 allowance_exhausted + Retry-After on POST /v1/runs; register the surface (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 41-02-PLAN.md — Postgres balance and the full balance contract on all three adapters; Treasurer boundary/order/short-circuit/fail-closed/idempotency/concurrency proofs over a scripted store clock (wave 2)
- [x] 41-03-PLAN.md — Full treasurer.allowance grammar (tenants, lifetime, warn_at, operator webhook target, strict keys, env overrides, redacted secret), D-11 boot coherence and the Treasurer wired in build_run_api (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 41-04-PLAN.md — Gate fork and the HTTP agent execute/stream/jobs routes, Treasurer attached to AgentApiState, Admin bound, OpenAPI 429 with the v0.9 golden exception (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 41-05-PLAN.md — Schedule-fired runs: migration 010 creator columns, RunSchedule/CreateRunSchedule.created_by, SubmitRun.attributed_to, SkipReason::AllowanceExhausted, role check unchanged (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 41-06-PLAN.md — Once-per-window notices: migrations 011/012, TreasuryNoticePort on three adapters with the sixteen-way race clause, warn-crossing claim with confirm/abandon (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 41-07-PLAN.md — TraceEvent::AllowanceWarning emitted by the worker on first dispatch, one herald line in all three heralds, the HTTP agent path through RunScope (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 41-08-PLAN.md — Operator webhook notice through the existing delivery queue: RunEventKind::AllowanceWarning, operator signing secret, boot-time SSRF check, the integrated warn-path proof (notice row + operator webhook + trace event + herald line once per window), rollout docs (wave 7)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 41-09-PLAN.md — ADR-0056, PROMOTION.md to 0057, facade re-export, consolidated register row, crate CHANGELOGs, public-API baseline, full phase gate (wave 8)

### Phase 42: Mid-Run Halt & SSE Terminal Status

**Goal**: A run that would overspend mid-flight stops cleanly and resumably instead of running
unchecked, and the SSE stream reports the run's real terminal status.
**Depends on**: Phase 38 (attachment-point ADR), Phase 41 (Treasurer facade's admission slice)
**Requirements**: ALLOW-03, ALLOW-05, PLAT-09
**Success Criteria** (what must be TRUE):

  1. A run in flight whose next draw would overspend halts cleanly — typed Treasurer error, status
     `Halted`, last checkpoint kept — on both an engine-driven (`WarEngine`) run and an agent-loop
     (`PaladinExecutionService`) run.

  2. A halted run resumes and completes once the allowance is replenished or the window resets,
     continuing from its last checkpoint rather than re-executing.

  3. The Treasurer derives the per-run `TokenBudget` from the remaining allowance and works
     alongside `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary without replacing
     any of them; a guard test keeps `Treasurer` a framework-only word.

  4. The SSE `done` event matches the persisted run status — `Cancelled` for a caller-cancelled run,
     `Halted` with the Treasurer reason for a spend halt.
**Plans**: 12 plans
**Research flag**: yes — mid-run halt has no prior art among surveyed comparable systems (none
have durable multi-step runs); the streaming settle-on-nonexistent-usage pitfall needs explicit
handling by reusing `TokenBudget`'s existing cutoff mechanism rather than inventing a second one.

Plans:
**Wave 1**

- [x] 42-01-PLAN.md — Design gate on the consolidated mid-run halt design (G1..G17 against D-05/D-06/D-10/D-12/D-13/D-14/D-18), then ADR-0057, the ADR-0056 dated note and PROMOTION.md to 0058 (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 42-02-PLAN.md — Tracer: HaltReason, the SpendGuard port, the engine boundary check with a typed HaltCause, the shared Treasurer evaluation and guard, worker attachment and map_outcome, reordered build_run_api, engine_spend_halt_tracer through the real router and SQLite; register the surface (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 42-03-PLAN.md — Migration 013 runs.halt_reason on three adapters, reason written before the status flips, GET /runs halt_reason + final_waypoint_id from one wire builder, caller halted-webhook key (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 42-04-PLAN.md — Guard hardening (exact-ceiling boundary, every-boundary reads, fail-closed ledger_unavailable, sticky child halts, no guard when unattributed) and resume by fork end to end with the documented recipe (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 42-05-PLAN.md — RunFinished.halt_reason from the engine, SSE done halt_reason identical on the live, degraded and replay paths (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 42-06-PLAN.md — RunFinishStatus::Cancelled, PerRunCancelProbe for same-instance cancels, drain emits no done, the cause-to-status invariant test (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 42-07-PLAN.md — Derived agent budget (dearest axis, i128), admit_for_model and the unpriced refusal, StopReason::AllowanceHalted, the one Treasurer-aware TokenBudget (tightest wins) proven to compose with ModelCallLimit, ToolCallLimit and the Commissary; register the surface (wave 7)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 42-08-PLAN.md — TokenBudget on every per-agent service, priced Treasurer at boot, HTTP agent routes admit with the model and answer allowance_halted + halt_reason (buffered stream done included), 422 model_unpriced, v0.9 golden exception (wave 8)

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 42-09-PLAN.md — Worker agent-kind halt (re-derive at dispatch), Treasurer-only TokenBudget on the shared service, agent-kind admission at POST /runs, WINDOWS.md D-08 row (wave 9)

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 42-10-PLAN.md — Migration 014 notice_kind (the notice store's halt rung) and the allowance_halted operator webhook event, signing and payload; register the surface (wave 10)

**Wave 11** *(blocked on Wave 10 completion)*

- [x] 42-11-PLAN.md — The boundary guard's notice legs: mid-run warn on the run's own stream and once-per-window halt notices to the operator, proven end to end (wave 11)

**Wave 12** *(blocked on Wave 11 completion)*

- [x] 42-12-PLAN.md — Herald halt line, the Treasurer vocabulary guard test, remaining WINDOWS.md rows, crate CHANGELOGs, full phase gate (wave 12)

### Phase 43: Rate Pacing

**Goal**: Outbound LLM calls back off on provider 429s instead of thrashing, in-process and across
the whole worker fleet, without ever leaving a run unpaced.
**Depends on**: Phase 38 (design seams only) — parallel-schedulable alongside Phases 39-42
**Requirements**: PACE-01, PACE-02, PACE-03, PACE-04, PACE-05
**Success Criteria** (what must be TRUE):

  1. `LlmError::RateLimitExceeded` carries a retry delay parsed from `Retry-After` (delta-seconds or
     HTTP-date) and from provider rate-limit headers, with header names verified against official
     OpenAI and Anthropic documentation.

  2. A mocked 429 against any `FallbackLlmAdapter` hop proves back-off with jitter — the retry delay
     is honored as a minimum — rather than immediate-retry thrash.

  3. With Redis configured, a 429 on one worker measurably slows every worker sharing that
     provider/model's pacing state, through an atomic Lua script using the Redis server clock.

  4. A distributed stampede lock (set-if-absent with expiry, fencing token, delete-only-if-owner)
     stops concurrent workers from issuing the same cached request twice.

  5. When Redis is unavailable, pacing degrades to conservative per-process pacing with a trace
     warning; a run is never left unpaced — covered by a test.
**Plans**: 13 plans
**Research flag**: yes — no adjacent in-tree pattern to imitate for the stampede lock, and the
fail-open-vs-fail-closed decision on Redis unavailability is a genuine open design question the PRD
does not resolve; needs deeper phase-specific research.

Plans:
**Wave 1**

- [x] 43-01-PLAN.md — Tracer: CadencePort + CadencePolicy, InMemoryCadence, CadenceLlmAdapter composed as Pricing(Cadence(provider)), OpenAI surfaces its first 429, treasurer.cadence, build_cadence/compose_llm at the agent host, end-to-end cadence_tracer test through a real OpenAI adapter; register (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 43-02-PLAN.md — Typed retry delay: RateLimitExceeded { retry_after, hints } + RateLimitHints, workspace sweep, the decorator honours the provider delay as a minimum and surfaces a delay beyond max_wait_secs (D-06); D-27 + register (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 43-03-PLAN.md — Shared rate-limit header parser (Retry-After both forms, retry-after-ms, OpenAI x-ratelimit-*, Anthropic anthropic-ratelimit-*), map_http_status_with_hints, OpenAI/Anthropic header snapshots, Anthropic first 429, quota-class 429s to UsageLimitExceeded (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 43-04-PLAN.md — Compat engine, DeepSeek and Gemini surface their first 429 with Retry-After (D-02 extended); conformance case 10 (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 43-05-PLAN.md — Shared cadence contract suite, bounded in-process state, stream gating, waiter spread, cancellation safety (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 43-06-PLAN.md — Fallback chains pace first and hop last (D-03): FallbackLlmAdapter::with_cadence, ModelFallbackMiddleware::paced, AgentRuntimeDeps.cadence; success-criterion-2 tests (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 43-07-PLAN.md — RedisCadence behind redis-cadence: server-clock Lua, lazy timed connection, live contract suite, two-worker fleet proof, CI job redis-cadence-integration (wave 7)

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 43-08-PLAN.md — ResilientCadence degraded mode (D-05): in-process fallback x degraded_multiplier, one warning per outage, probe back-off, recovery; dead-Redis and never-unpaced tests (wave 8)

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 43-09-PLAN.md — treasurer.cadence redis backend + env overrides, build_cadence resilient branch, every server composition root paced from one shared wiring, operator docs (wave 9)

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 43-10-PLAN.md — Stampede lock primitives: CadencePort::try_lock/unlock, FencingToken (Distributed/Local), in-memory/Redis/resilient fail-open, defaulted NodeCachePort::put_fenced with the RedisNodeCache fenced write (wave 10)

**Wave 11** *(blocked on Wave 10 completion)*

- [x] 43-11-PLAN.md — Engine stampede lock: WarEngine::with_cadence, lock loop around the cache miss-to-put window, release on every exit, cancellation-aware waiting (wave 11)

**Wave 12** *(blocked on Wave 11 completion)*

- [x] 43-12-PLAN.md — Provider header verification against official OpenAI/Anthropic docs with an operator checkpoint for what the sandbox cannot fetch; verified-source rustdoc (wave 12, non-autonomous)

**Wave 13** *(blocked on Wave 12 completion)*

- [x] 43-13-PLAN.md — ADR-0058, Cadence in the ubiquitous language, crate CHANGELOGs, D-27 semver measurement, full phase gate and manual credential review (wave 13)

### Phase 44: Legacy Clean-Break Removal

**Goal**: The superseded Battalion error/retry/timeout surfaces and the untyped `LlmError` string
variant are gone from the tree, with every removal recorded.
**Depends on**: Phase 43 (sequenced after the Treasurer feature phases to avoid diff conflicts with
concurrently touched files)
**Requirements**: LEGACY-01, LEGACY-02, LEGACY-03, LEGACY-04
**Success Criteria** (what must be TRUE):

  1. `battalion::RetryPolicy`, `battalion::ErrorStrategy` and `battalion::NodeError` no longer
     exist, while the same-named Aegis and Flow types are untouched and still compile.

  2. Formation, Phalanx and Campaign no longer carry their own legacy timeout handling — Aegis
     timeouts are the only timeout path for those patterns.

  3. `PaladinError::LlmError(String)` no longer exists, and `conclave_execution_service.rs`'s retry
     check uses the typed `LlmFailure`/`Transience` taxonomy instead of string matching.

  4. The X-03 supersession is recorded in an ADR, and every removal has a `MIGRATION.md` §9.2 row,
     a `cargo semver-checks` allowlist entry, and updated examples/docs.
**Plans**: 13 plans

Plans:
**Wave 1**

- [x] 44-01-PLAN.md — Tracer: BattalionConfig.aegis + validate_aegis, the one crate-private per-attempt runner (aegis_attempt), Campaign on it with its whole-run wrapper deleted, end-to-end Commander test; runner edge matrix (wave 1)
- [x] 44-02-PLAN.md — ADR-0059 (Phase 44-only X-03 supersession + replacement design), ADR-0001/0002 Superseded, PROMOTION next free 0060, PROJECT.md rows (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 44-03-PLAN.md — Circuit breaker and Conclave predicate on transience() (Transient only); PaladinError::LlmError, is_retryable, is_terminal removed with every Rust site and rustdoc (wave 2)
- [x] 44-04-PLAN.md — Formation on the runner: per-attempt timeouts, Aegis retry, on_error Absorb / fail fast, structured timeout errors; Formation-routed tests migrated (wave 2)
- [x] 44-05-PLAN.md — Phalanx on the runner: typed failures in declaration order, independent per-Paladin retry, cancellation, no string parsing; Phalanx tests migrated (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 44-06-PLAN.md — One-way node_errors retype to the structured NodeError behind a checkpoint:decision, heralds' richer line, shape and deserialization-break tests (wave 3, non-autonomous)
- [x] 44-07-PLAN.md — Commander without its execute() wrapper, aegis-sourced Conclave/Maneuver bridges, 300 s per-attempt builder default; Conclave on the runner with the engine backoff (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 44-08-PLAN.md — CLI (YAML keys unchanged, per-attempt), the eight examples and the doc-examples anchor show the Aegis equivalents (wave 4)
- [x] 44-09-PLAN.md — mdBook pages and DEMOS.md on Aegis and the typed taxonomy; per-attempt consequences stated; fictional battalion: YAML section replaced (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 44-10-PLAN.md — Legacy retry module and facade re-export deleted (API baseline refreshed), last legacy writers swept (incl. the two root Campaign / chain-of-command tests), legacy definitions deleted (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 44-11-PLAN.md — MIGRATION.md 9.1 rows M-B-06.., 9.4 stored-JSON note, D-27 measurement, Phase 44 9.2 rows, allowlist block and crate allow lines (wave 6)
- [x] 44-13-PLAN.md — tests/legacy_removal_guard.rs: source-tree guard over every Phase 44 removal with two positive controls, repository scan clean (wave 6, split out of 44-10)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 44-12-PLAN.md — CHANGELOG Removed/Changed (D-21), ADR-0059 conforms, full phase gate and manual credential review (wave 7)

### Phase 45: RustFS Swap & Platform/Observability Deviations

**Goal**: The dev/test/CI object store runs on a maintained, pinned image instead of a terminal
MinIO pin, and two standing accepted deviations (legacy-agent SSE/webhook silence, tracing
overhead) are closed or re-measured.
**Depends on**: none functionally -- resequenced on 2026-09-29 to execute ahead of Phases 41-44 (Phase 45 D-01): the earlier "Phase 44" ordering was a diff-conflict-avoidance preference, and every later phase's Coverage / Integration / Docker Integration / Kubernetes Smoke evidence is red until the object store is replaced; functionally independent of the Treasurer phases
**Requirements**: STORE-01, STORE-02, STORE-03, PLAT-08, OBS-05
**Success Criteria** (what must be TRUE):

  1. The dev/test compose stack and the Coverage, Integration Tests, Docker Integration Tests and
     Kubernetes Smoke Test CI jobs run against RustFS pinned to an exact tag, bucket bootstrap
     replaces `mc`, and no MinIO image remains in any live configuration.

  2. The `FileStoragePort` contract suite (presigned URLs, multipart uploads, ETags) passes against
     RustFS using the existing S3 adapter; a new adapter behind a feature flag is added only if
     that suite fails.

  3. Storage docs record the decision on whether the production Kubernetes manifest also moves to
     RustFS.

  4. A legacy `Runnable::Agent` run emits SSE live events and webhook deliveries the same way a
     graph run does.

  5. `LogTraceSink`/`TraceDispatcher::emit` skip serialization when logging is disabled and reuse
     their buffers; the tracing-overhead benchmark is re-measured and either meets PRD 07's ≤3% bar
     or a new accepted figure is recorded.
**Plans**: 7 plans

Plans:
**Wave 1**

- [x] 45-01-PLAN.md — Wave 1 (tracer, STORE-02): make the FileStoragePort contract suite real and green against RustFS with the reused adapter (path-style bucket bootstrap, multipart via a stateless token, ETag + exercised presign cases, harness fixes, RustFS `GenericImage` local mode, D-01 roadmap line)
- [x] 45-02-PLAN.md — Wave 1 (PLAT-08): legacy `Runnable::Agent` runs stream live SSE and enqueue webhooks through the shared machinery; single terminal event; `persist_failure`; row 31 closed via the ledger tool
- [x] 45-03-PLAN.md — Wave 1 (OBS-05): bench harness (point A), `LogTraceSink` enablement guard (point B), per-thread buffer + `CompositeSink` last-child move (point C), evidence protocol

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 45-04-PLAN.md — Wave 2 (STORE-01/02): CI Integration/Coverage/Docker Integration/E2E/k8s smoke on `rustfs/rustfs:1.0.0`, `mc` deleted, suite runs with a tests-ran check, `k8s/minio.yaml` → `k8s/rustfs.yaml`
- [x] 45-05-PLAN.md — Wave 2 (STORE-01): dev compose, devcontainer, Makefile, `.env.example`, coverage probe and dev docs on RustFS

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 45-06-PLAN.md — Wave 3 (STORE-03): ADR-0055, storage docs reframed with the production-manifest decision, phase-wide no-MinIO gate, CHANGELOG, MinIO todos closed

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 45-07-PLAN.md — Wave 4 (OBS-05, non-autonomous): maintainer-run three-point bench re-measure; D-19 meet/amend recording (evidence, row 35 ledger row, docs, CHANGELOG)

**Research flag**: yes — RustFS's S3 API-surface parity (presigned URLs, multipart uploads, ETag
format) is MEDIUM-confidence only; the phase's first work item should close this via the existing
`FileStoragePort` contract-test suite before any adapter code is written.

### Phase 46: Docs Currency & Hygiene

**Goal**: The v0.10.0 audit's docs debt is closed, v0.11.0's own surface is documented, and the
milestone's validation/tech-debt bookkeeping is current.
**Depends on**: Phase 45 and Phase 48 (documents everything the milestone shipped, so it runs
last among feature work; the v0.10 → v0.11 migration guide must carry Phase 48's
`Node<T>` → `Entity<T>` rows — added 2026-10-12), and Phase 49 (the consumer-surface muster and its `MIGRATION.md` consumer note are inputs to the v0.10 → v0.11 migration guide — added 2026-10-14), and Phase 50 (the ADR amending 0004, the facade re-exports, the typed refusal and the "Switching providers" mdBook page are inputs to the docs pass and the migration guide — added 2026-10-14), and Phase 51 (the ADR amending 0004 a second time, the `ReasoningEffort` public type, its MIGRATION §9.2 rows and the "Reasoning effort" mdBook page are inputs to the docs pass and the migration guide — added 2026-10-14), and Phase 50.1 (the §9.8 consumer-guide additions — pre-`v0.10.0` pointers, 429/Cadence, decorator forwarding, context-window consequence — the harmonised `*Config::new` §9.2/§9.8 rows and the facade Aegis exports are inputs to the v0.10 → v0.11 migration guide, as is web3sec's adaptation feedback after 50.1 seals — added 2026-10-19)
**Requirements**: CURR-22, CURR-23, CURR-24, CURR-25
**Success Criteria** (what must be TRUE):

  1. The v0.10.0 acceptance-audit docs fixes are closed: "twelve publishable crates" appears in
     `development-setup.md`, `release-automation.md` and `release-recovery.md`; `paladin-eval` has
     Trusted Publishing and Credential History rows; the CHANGELOG `[0.10.0]` heading date is
     corrected.

  2. A Treasurer mdBook page, a configuration reference for pricing/`allowance`/the warn threshold,
     and a v0.10 → v0.11 `MIGRATION.md` guide are published and reachable from the book's
     navigation.

  3. Nyquist validation for Phases 22, 24, 29, 30, 34, 36 and 36.1 is out of `draft`, and Phase 28
     is either `nyquist_compliant` or its gaps are explicitly recorded.

  4. Each of the three v2 debt lines (oversized service files, clone/lock contention,
     dependency-allowlist drift) is fixed or carries a written waiver.
**Plans**: TBD

### Phase 47: v0.11.0 Crate Release

**Goal**: v0.11.0 ships to crates.io with every gate green.
**Depends on**: Phase 46 (needs the milestone's docs/hygiene current before release); Phase 49 (the consumer-surface muster is green before v0.11.0 is tagged — added 2026-10-14; the consumer bump itself was moved to a single post-release move to `v0.11.0`, scope cut 2026-10-14); Phase 50 (the provider-swap matrix, the `api-surface` refresh for the nine facade exports and their CHANGELOG/MIGRATION entries land before the tag — added 2026-10-14); Phase 51 (the `ReasoningEffort` type, the `UnsupportedParameter` variant and the per-provider effort config key are additive public surface that the `api-surface` refresh and the CHANGELOG/MIGRATION entries must carry before the tag — added 2026-10-14); Phase 50.1 (the harmonised `*Config::new` break, the facade Aegis exports and the extended muster land and the `api-surface` baseline is refreshed before the tag — added 2026-10-19)
**Requirements**: SHIP-07
**Success Criteria** (what must be TRUE):

  1. All 12 publishable crates are live on crates.io at version `0.11.0`.
  2. The release tag is cut on a `main` merge commit, per the two-SHA rule.
  3. CHANGELOG and MIGRATION.md are complete for v0.11.0, and the publish-order gate is green on
     the release run.
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
| 22-37.1 | v0.10.0 | 231/231 (228 executed, 3 superseded) | ✅ Shipped | 2026-09-23 |
| 38. Design Seams & Pricing/Cost Producer | v0.11.0 | 9/9 | Complete    | 2026-09-26 |
| 39. Spend Ledger | v0.11.0 | 8/8 | Complete    | 2026-09-28 |
| 40. Tenant Identity & Run-Read Scoping | v0.11.0 | 6/6 | Complete    | 2026-09-30 |
| 41. Admission-Time Allowance Enforcement | v0.11.0 | 9/9 | Complete    | 2026-10-12 |
| 42. Mid-Run Halt & SSE Terminal Status | v0.11.0 | 12/12 | Complete    | 2026-10-07 |
| 43. Rate Pacing | v0.11.0 | 13/13 | Complete    | 2026-10-09 |
| 44. Legacy Clean-Break Removal | v0.11.0 | 13/13 | Complete    | 2026-10-11 |
| 45. RustFS Swap & Platform/Observability Deviations | v0.11.0 | 7/7 | Complete    | 2026-09-30 |
| 46. Docs Currency & Hygiene | v0.11.0 | 0/TBD | Not started | - |
| 47. v0.11.0 Crate Release | v0.11.0 | 0/TBD | Not started | - |
| 48. Base Entity Rename (runs after 44, before 46-47) | v0.11.0 | 9/9 | Complete    | 2026-10-14 |
| 49. Downstream Adoption of v0.10.1 (web3sec Consumer Bump) (runs after 48, before 46-47) | v0.11.0 | 3/3 | Complete    | 2026-10-14 |
| 50. Provider Parity & Configuration-Driven Model Swap (runs after 49, before 46-47) | v0.11.0 | 32/32 | Complete    | 2026-10-19 |
| 50.1 Consumer-Surface Readiness for v0.11.0 (INSERTED; runs after 50, before 51) | v0.11.0 | 9/9 | Complete    | 2026-10-20 |
| 51. Adjustable Reasoning Effort (runs after 50.1, before 46-47) | v0.11.0 | 0/TBD | Not started | - |

**v0.8.0 shipped 2026-08-24:** 14 phases, 149 plans, 65/65 requirements, 1,014 commits
(`be2ff05..48ac11a5`). Audit status `tech_debt` — no blockers; see
[`milestones/v0.8.0-MILESTONE-AUDIT.md`](milestones/v0.8.0-MILESTONE-AUDIT.md).

**v0.9.0 shipped 2026-09-01:** 4 phases, 25 plans, 20/20 requirements, 240 commits
(`48ac11a5..3957d701`). Audit status `tech_debt` — no blockers; see
[`milestones/v0.9.0-MILESTONE-AUDIT.md`](milestones/v0.9.0-MILESTONE-AUDIT.md). Tag `v0.9.0`
cut post-close (2026-09-01) on merge commit `0b5d4106`: release run `33542459191` fully green,
all eleven crates on crates.io at `0.9.0` — the first stable release since 0.5.1, reconciling
release numbers with milestone names. See MILESTONES.md.

**v0.10.0 shipped 2026-09-23:** 19 phases (22-37.1), 231 plans (228 executed, 3 superseded),
88/89 requirements (SHIP-05 superseded by SHIP-06), 1,678 commits (`495483ef..6a08c293`,
2026-09-01 → 2026-09-23). Audit status `tech_debt` — no blockers; see
[`milestones/v0.10.0-MILESTONE-AUDIT.md`](milestones/v0.10.0-MILESTONE-AUDIT.md). Released as
two tags: `v0.10.0` on merge commit `1d4a9724` (2026-09-18) published 3 of 12 crates before a
publish-order defect stopped the pipeline; `v0.10.1` on merge commit `f7dae267` (2026-09-21,
release run `35659477719`) published all twelve, and the three `0.10.0` orphans were yanked.
Closeout type `override_closeout` (0 unverified phases; 3 superseded Phase 37 plans; 2 open
artifacts acknowledged). See MILESTONES.md.

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

### Phase 48: Base Entity Rename

**Goal**: The word "node" means exactly one thing in the Paladin workspace — a vertex in a
`WarGraph`/Campaign graph — because the original base-layer entity wrapper
`paladin_core::base::entity::node::Node<T>` (uuid + created/modified + optional name + payload +
`version` flag; the thing `Paladin`, `Council`, `Grove`, `User`, `Log`, `ContentItem` and
`PromptItem` are built on) is renamed to `Entity<T>` with its payload field `node` renamed `data`,
and the unused remainder of the base layer is dispositioned.
**Depends on**: Phase 44 (complete 2026-10-11 — it removed the legacy
`battalion::NodeError { node_name, error }` summary, so exactly one `NodeError` remains and the
Battalion files are quiet). **Resequenced ahead of Phases 46 and 47** on 2026-10-12: it is a
public-API break that must land before the v0.10 → v0.11 `MIGRATION.md` guide (Phase 46,
CURR-23) is written and before v0.11.0 is tagged (Phase 47).
**Requirements**: ENTITY-01, ENTITY-02, ENTITY-03, ENTITY-04, ENTITY-05
**UI hint**: no
**Ground truth** (code survey 2026-10-10, re-verified on the post-Phase-44 tree `25dc267` on
2026-10-12, recorded in `48-CONTEXT.md`): the graph sense of "node" arrived with the v0.10.0
superstep engine (Phases 22-25) and is now ~5× larger than the entity sense — `NodeId` 1,910
occurrences / 91 files, `NodeSpec` 1,034 / 64, `node_id` 1,151 / 85,
plus `StateNode`, `NodeContext`, `NodeCache*`, `NodeInterceptor`, `NodeExecutionRecord`,
`NodeOutcomeKind`, `NodeProgressKind` — and it is in public wire formats
(`docs/schemas/wargraph-doc.schema.json`, `TraceEvent::NodeStarted`/`NodeProgress`, persisted
`Waypoint`s, Redis node-cache keys, the `node_cache` config section). The entity sense is
`base::entity::node` imported in 117 `.rs` files, `Node::new` 334 occurrences, `.node` payload
access 739 occurrences / 107 files, 9 mdBook/README files, 13 example/doc-example/bench files
(Phase 44 touched none of the base layer or the eight `Node<T>`-based container files).
Only 8 of the 70 files in `platform/container/` use `Node<T>`; everything newer (`Run`,
`Assistant`, `Waypoint`, `VaultRecord`, `Battlefield`, traces, the treasury ledger) carries its own
typed id and `schema_version`. The two senses already collide in `NodeSpec::Paladin { paladin:
Box<Paladin> }` (a graph node wrapping an entity Node), in `paladin.node` vs `graph.node(id)` vs
`progress.node == NodeId::new(..)` inside the same engine files, and — until Phase 44 removed the
legacy one on 2026-10-11 — in two unrelated types both named `NodeError`. `Paladin` is aliased twice (`paladin.rs:229` and `citadel.rs:48`). The
revisioning the base layer was designed for is only wired for `ContentItem`
(`NodeVersionService` via the root crate's `content_service.rs`); `Assistant` versioning (Phase 27)
was built independently. `Field`, `Collection`, `Element`, `FieldVersionService` and
`CollectionVersioningService` have zero consumers outside `base/`.
**Success Criteria** (what must be TRUE):

  1. `paladin_core::base::entity::Entity<T>` exists with the payload field named `data`, and
     `Paladin`, `Council`, `Grove`, `User`, `Log`, `ContentItem` and `PromptItem` are built on it;
     `#[serde(alias = "node")]` on the payload field means every JSON persisted before the rename
     (Citadel snapshot files, stored Paladins, and any caller-owned store of entity JSON -- no in-tree
     SQL table stores entity JSON) still deserializes, proven by a
     fixture test against pre-rename JSON.

  2. The old name's fate is decided and recorded, one of two ways: **(default)** `Node<T>` and
     the `base::entity::node` module are removed outright and an ADR (0060) records the
     Phase-48-only X-03 supersession in the ADR-0059 shape, with `MIGRATION.md` §9.7 still
     empty; **or** a `#[deprecated(since = "0.11.0")]` `Node<T>` alias and re-exporting module
     keep compiling with no in-tree use, and §9.7 schedules their removal for v0.12.0. Either way
     the choice is in `48-CONTEXT.md` D-02 before planning.

  3. Phase 44 (sealed 2026-10-11) left exactly one type named `NodeError` (the structured
     `platform::container::node_error::NodeError`); after Phase 48 that is still true,
     `tests/legacy_removal_guard.rs` is still green, there is exactly one `Paladin` alias (the
     `citadel.rs:48` duplicate is gone), and `NodeVersion`/`NodeVersionRepository`/
     `NodeVersionService`/`InMemoryNodeVersionRepository` are renamed `EntityVersion*`.

  4. Each of `Field`, `Collection`, `Element`, `FieldVersionService` and
     `CollectionVersionService` (module `collection_versioning_service`) is either removed or kept with
     a written reason, and the `Entity<T>::version` flag's contract is stated in rustdoc (what
     revisioning it promises, and that only `ContentItem` currently honours it). Phase 48 outcome:
     all five retained pending review, the reason written in `base/mod.rs` rustdoc (D-05, ADR-0060).

  5. The graph vocabulary (`NodeId`, `NodeSpec`, `StateNode`, `NodeContext`, `NodeCache*`,
     `TraceEvent::Node*`, `wargraph-doc.schema.json`, `node_cache` config) is byte-for-byte
     untouched; an ADR records that "node" is reserved for the graph sense and "Entity" for the
     base wrapper, the CLAUDE.md/copilot-instructions naming table and the mdBook domain-model,
     design-patterns and crate-map pages say the same; a repository-scan test in the
     `tests/legacy_removal_guard.rs` shape (positive controls included) pins both that freeze and
     the absence of the old entity paths; every rename has a `MIGRATION.md` §9.2 row
     and a set-equal `.cargo/semver-checks-allowlist.toml` entry, the gated public-API baseline
     `.project/current-exports.txt` is refreshed via `make api-surface-update` with a CHANGELOG entry,
     and `cargo doc`, `make
     check-api-examples`, `make clean-code` and the 82 % coverage floor stay green.

  *Corrected 2026-10-14 (plan 48-09): criteria 1, 4 and 5 named a storage list, a type name and a baseline file that the shipped tree and 48-CONTEXT.md D-05/D-15 settle differently. No requirement ID changed.*
**Plans**: 9 plans

Plans:
**Wave 1**

- [x] 48-01-PLAN.md — Tracer: capture the five v0.10 fixtures (Paladin, Citadel snapshot, ContentItem, PromptItem, version record) from the unmodified tree; a pre-rename snapshot loads through FileCitadel; the compat harness over all five shapes
- [x] 48-02-PLAN.md — ADR-0060 (Phase-48-only X-03 supersession, Entity design, citadel unification, five base items retained pending review, node/Entity vocabulary rule); PROMOTION next free 0061; PROJECT.md rows; residual note in the content-system review todo

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 48-03-PLAN.md — Version family renamed EntityVersion* with node_id/node_data read-aliases (D-09); retained-items reason in base/mod.rs rustdoc (D-05); API baseline refreshed (14 lines)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 48-04-PLAN.md — Citadel placeholder PaladinData/PaladinStatus unified with the real ones (D-10); Stage A: Node<T> -> Entity<T> at base::entity::Entity (D-01, D-02, D-13); entity-sense Rust prose finished

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 48-05-PLAN.md — Stage B: payload node -> data and ContentItem/PromptItem node -> entity with read-aliases (D-03, D-08); alias edge-case tests; Entity<T> revisioning and serialization rustdoc (D-04, D-11)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 48-06-PLAN.md — Docs agree: mdBook domain-model/design-patterns/crate-map/overview/stable-api, LayerArchitecture.svg, READMEs, CLAUDE.md and copilot naming-table Entity row (D-07)
- [x] 48-07-PLAN.md — Register once, measured (D-14): D-27 semver diagnostic, MIGRATION 9.2 rows + set-equal allowlist + new allow lines in one commit; 9.1 M-B-12, 9.4, 9.7 sentence; CHANGELOGs

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 48-08-PLAN.md — tests/entity_rename_guard.rs: old entity paths absent, one Paladin/PaladinData/PaladinStatus/NodeError/Entity, graph definitions pinned, retained paths not banned (D-07); byte-identical graph freeze

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 48-09-PLAN.md — Closeout: full gate run with measured coverage; ADR-0060 conforms; ENTITY-01/04/05 and ROADMAP wording corrected (ENTITY-04 retained pending review), IDs unchanged

### Phase 49: Downstream Adoption of v0.10.1 (web3sec Consumer Bump)

**Goal**: Web3 Security Paladin ("web3sec"), Paladin's only consumer, moves from its pre-`v0.10.0`
submodule pin (`35fd839`) onto the `v0.10.1` release tag (`f7dae267`) in one coordinated step, per
Milestone 13 §5 (clean break, no shims, coordinated pointer bump). The three documented breaks it
hits (`MIGRATION.md` §9.2 — B-1 `Commissary::new`/`from_port` lost `is_exact_counter`; B-2
`PaladinResult.token_count: u32` → `usage: TokenUsage`; B-3 `TokenUsage` gained three
`Option<u32>` fields) are adopted on the consumer side with behaviour preserved, and Paladin gains
a compile-only **consumer-surface muster** — a test that `use`s every path and signature the
consumer imports — so the next downstream break is a red Paladin check, not a surprise at bump
time. No Paladin library API changes to accommodate the consumer.
**Scope cut (2026-10-14, owner decision)**: the phase closes with its Paladin-side half only. Plans 49-01..03 landed the muster (`tests/consumer_surface_muster.rs`, `[[test]]` table, `make check-consumer-surface`), the `MIGRATION.md` §9.8 `### Library consumers: v0.10.1 → v0.11.0` note and the red proof (`49-MUSTER-RED-EVIDENCE.md`). Plans 49-04..06 were cancelled and their files removed: the handoff of 2026-10-08 targets `v0.10.1`, but the tree the consumer meets next is `v0.11.0` with the Phase 44 and 48 breaks already in (the research counted roughly three times the handoff's break list), so moving web3sec to `v0.10.1` now would break it again weeks later and contradict the Milestone 13 §5 single coordinated bump the handoff itself cites. ADOPT-01..03 are superseded by one consumer move straight to `v0.11.0` after Phases 50 and 51, with the §9.8 note as its spec (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`). No `v0.10.2` is cut.
**Depends on**: Phase 48 (complete 2026-10-14; the muster is written against the post-rename
tree). **Sequenced ahead of Phases 46 and 47** (owner decision 2026-10-14): the first of a series
of consumer-driven phases that must land before the v0.11 docs pass and tag, because the muster
and the `MIGRATION.md` consumer note change what Phase 46's migration guide must say.
**Requirements**: ADOPT-01, ADOPT-02, ADOPT-03, ADOPT-04, ADOPT-05
**UI hint**: no
**Source**: the web3sec handoff of 2026-10-08, filed verbatim as `49-HANDOFF-web3sec.md` with
its companion `49-REFERENCE-adoption.diff` (15 files, +39/−34 — the trial adoption that builds
and passes 2,592 / 0 / 2 on `v0.10.1`; mechanical, unreviewed, and using the D-2 fallback). The
consumer repository is not reachable from this checkout: ADOPT-01..03 are executed in web3sec
against handoff §7 as the spec and are closed here by the §9 return report; ADOPT-04 and
ADOPT-05 are Paladin-side and executable here.
**Ground truth** (verified 2026-10-14 on `e8815625`, 180 commits past `v0.10.1`, workspace still
`version = "0.10.1"`): B-1, B-2 and B-3 read exactly as the handoff states —
`Commissary::new(provider, capabilities, counter, config)` / `from_port(llm, counter, config)` in
`crates/paladin-llm/src/services/commissary.rs`; `TokenUsage`
(`paladin_core::platform::container::token_usage`, re-exported by
`paladin_ports::output::llm_port`) has six fields, derives `Default`, is not `#[non_exhaustive]`,
and offers `new(prompt, completion)` plus `with_cache_read` / `with_cache_write` /
`with_reasoning` but **no total-only constructor**, so D-2's lossless route is the only one that
keeps the documented `total == prompt + completion` invariant. **Drift the handoff cannot see:**
the tree web3sec meets at its *next* bump already differs from `v0.10.1` on four Appendix-A
items — `battalion::ErrorStrategy` and the legacy `battalion::NodeError` summary are removed
(Phase 44, §9.2 rows; the structured `platform::container::node_error::NodeError` remains),
`core::base::entity::node::Node` is removed in favour of `base::entity::Entity<T>` with payload
`data` (Phase 48, §9.2 row), and `PaladinResult` gained `cost: Option<Cost>` (Phase 38,
`#[serde(default)]`), which breaks web3sec's full struct literal in `phalanx_dispatch.rs` again.
Appendix A also lists `mcp_protocol::looks_like_auth_failure`, which is a private `fn` at both
`v0.10.1` and `HEAD` (its rustdoc says it was ported *from* web3sec's `audit-mcp`), so the
imported surface must be re-verified from the consumer's actual `use` lines before the muster
freezes it. Precedent for a repository-level guard: `tests/legacy_removal_guard.rs`,
`tests/entity_rename_guard.rs`, `tests/treasurer_vocabulary_guard.rs` (std-only scans); the
muster is a different shape — a compile-time `use` + fn-pointer test — settled at discuss.
`MIGRATION.md` §9.8 is today a v0.9 → v0.10 *operator* checklist with no library-consumer
section.
**Success Criteria** (what must be TRUE):

  1. `git -C paladin rev-parse HEAD` in the consumer equals `f7dae267…`, the consumer commit
     records the pointer together with the adaptation and `Cargo.lock`, and the local-only
     submodule commit `4b7bed87` is recorded as superseded (its seven planning files are
     byte-identical in `v0.10.1`).

  2. The consumer gates pass on the new pin — `cargo fmt --check`, `cargo clippy --workspace
     --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` — with
     the test count ≥ the 2,592 / 0 / 2 baseline, no `unwrap`/`expect`/`panic!` added to library
     code, and `grep -rn 'is_exact_counter\|token_count: report' crates/` finding no stale call
     site (remaining prose mentions updated).

  3. The D-2 token-usage mapping is decided and recorded (lossless `TokenUsage` carried through
     `RoutineReport` preferred; the `total_tokens`-only fallback only with a site comment and a
     tech-debt entry), and the §9 return report (consumer SHA, D-2 choice, gate totals, Paladin
     commits, divergences) is filed in the phase directory.

  4. Paladin carries a compile-only consumer-surface muster covering every re-verified Appendix-A
     import path and signature as it exists on the current tree, green on `cargo test`, wired into
     the existing gates, and **proven red**: it fails to compile when (for example) a `bool` is
     re-added to `Commissary::new`.

  5. `MIGRATION.md` gains a consumer-facing upgrade note — the D-2 "no total-only `TokenUsage`"
     pitfall and the `v0.10.1` → `v0.11.0` break list the muster exposes (`ErrorStrategy`, legacy
     `NodeError`, `Node<T>` → `Entity<T>`, `PaladinResult.cost`) — and **no Paladin library API
     is changed, shimmed or deprecated** for the consumer: zero new §9.2 rows and zero
     `semver-checks` allowlist changes from this phase; `make clean-code`, `make check-gates`,
     `make api-surface` and the 82 % coverage floor stay green. No `v0.10.2` is cut.
**Plans**: 3 plans (49-04..06 cancelled 2026-10-14, scope cut)

Plans:
**Wave 1**

- [x] 49-01-PLAN.md — Tracer: the compile-only consumer-surface muster (`tests/consumer_surface_muster.rs`) green under `cargo test` as one `[[test]]` target with `required-features`; gate wiring proved (`make check`/`make lint`/`api-surface`), `make check-consumer-surface` alias outside check-gates; 49-VALIDATION.md map filled
- [x] 49-02-PLAN.md — MIGRATION.md §9.8 `### Library consumers: v0.10.1 → v0.11.0` (D-2 no-total-only-`TokenUsage` pitfall, DR-1..DR-11 break table, insulation advice) + CHANGELOG [Unreleased] entry; zero §9.2 rows, zero allowlist changes

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 49-03-PLAN.md — Proven red: `bool` re-added to `Commissary::new` (callers patched) and a dropped facade re-export each fail only the muster; tree restored; 49-MUSTER-RED-EVIDENCE.md

*Waves 3-5 (49-04 consumer hand-back, 49-05 return-report checkpoint, 49-06 muster v1.1) cancelled 2026-10-14 with the scope cut; the consumer's import inventory is re-verified at its `v0.11.0` bump instead (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`).*

### Phase 50: Provider Parity & Configuration-Driven Model Swap

**Goal**: Any Paladin agent can be moved from one LLM provider/model to another (Anthropic ⇄
OpenAI ⇄ DeepSeek ⇄ Kimi ⇄ Qwen ⇄ Grok ⇄ Gemini ⇄ Ollama ⇄ OpenAI-compatible) by **changing
configuration only**: the behaviour is equivalent, or the build fails early with an error naming
the provider, model and parameter. Today neither is true — web3sec's attempt to run its
Anthropic-backed agents on Moonshot `kimi-k3` fails or silently degrades in five blocking and
eleven degrading ways (handoff §5). The headline finding is that most blockers are cross-cutting
(capability modelling, configuration plumbing, fallback semantics) and several are defects in the
three original adapters, the worst being an OpenAI adapter that sends an **empty user message** on
the main execution path. This phase lands the red provider-swap matrix test and the OpenAI defect
first, then models parameter support per provider *and* per model under one documented policy (an
ADR amending ADR-0004), makes provider construction honour the `llm.<provider>` config blocks
through a public library API and exports all nine adapter/config pairs from the facade, seeds a
dated per-model profile registry that Commissary rations against, brings every adapter to the same
response/error mapping (empty output, typed refusal, usage cap, cache read/write, reasoning tokens
and text, stop sequences) with per-step models in fallback chains, and closes with all nine
adapters in the shared conformance suite and the credential-gated live smoke plus an mdBook
"Switching providers" page. Library-level APIs matter as much as server config: the consumer
builds adapters directly from their `*Config` types and never touches `agent_host`.
**Depends on**: Phase 49 (complete 2026-10-14, scope cut: the consumer-surface muster is green before this phase
changes the surface it musters, and catches this phase's own breaks; the consumer stays on its old
pin until it moves straight to `v0.11.0` after Phases 50 and 51). **Sequenced ahead of Phases 46 and 47** (owner decision 2026-10-14): the second of the
consumer-driven phases that must land in v0.11.0, because the ADR amending 0004, the facade
exports, the typed refusal and the "Switching providers" page change what Phase 46's docs pass and
migration guide must say. The companion reasoning-effort handoff (not yet filed in this repository)
is the natural next phase and depends on this one's parameter-support model and profile registry
(handoff §8), so Plans 2 and 4 are prerequisites for it.
**Requirements**: PARITY-01, PARITY-02, PARITY-03, PARITY-04, PARITY-05, PARITY-06, PARITY-07, PARITY-08
**UI hint**: no
**Source**: the second web3sec handoff of 2026-10-13, filed verbatim as
`50-HANDOFF-web3sec-parity.md` — a read-only audit of `origin/main` @ `df24689c` (post-`v0.10.1`)
with live Moonshot vendor facts dated 2026-10-07 and 2026-10-13 (`kimi-k3` 1,048,576-token window;
`thinking.effort` validated but top-level `reasoning_effort` silently ignored; unconfigured effort
`max` reasons for 492 s on a one-line question and returns empty content at `finish_reason:
length`; `temperature` must be omitted or exactly `1`). The handoff proposed "Phase 38" against
its own roadmap snapshot; it is Phase 50 here. Its §6 decisions **D-1** (policy for a configured
parameter the target does not accept — recommendation (b): refuse only an *explicit* value, skip
auto-selected ones with a `warn!` and a run-metadata entry), **D-2** (typed refusal — a new
`LlmError::Refusal` variant, `Permanent`, distinct from `ContentFilter`), **D-3** (per-model limits
— a static dated table plus config overrides now, live `/models` discovery optional later) and
**D-4** (facade default features stay `openai + anthropic + deepseek` per ADR-0046, the other six
behind their existing opt-in features) are settled at discuss. The handoff's §7 shape — one
phase, about six plans in the order red tests + OpenAI defect → parameter modelling → config-driven
construction → profile registry → response/error parity → conformance/smoke/docs — is the
starting point for planning, not a constraint.
**Ground truth** (verified 2026-10-14 on `83a16bfa`, 181 commits past `v0.10.1`, workspace still
`version = "0.10.1"`; the handoff's line numbers have drifted but every claim re-checked holds):
**B-4** — `crates/paladin-llm/src/openai/adapter.rs:435` builds the user turn from
`user_prompt.context.clone().unwrap_or_default()` while the execution service sets `query` and
`context: None` (`paladin_execution_service.rs:2785-2786`, `:2801`, `:2974`), so every OpenAI
call through `PaladinExecutionService` sends an empty user message. **B-1** — Kimi declares
`temperature_range: Some((1.0, 1.0))` (`kimi/adapter.rs:210`) and never sends `temperature`;
Anthropic declares `Some((0.0, 1.0))` (`anthropic/adapter.rs:711`) and also never sends it
(`temperature: None` at `:259`, `:303`, "deprecated for this model"); `PaladinBuilder` checks
both the manual and the auto-selected value against `temperature_range`
(`paladin_builder.rs:1141`, `:1332`), so an original adapter already discards a validated value
silently, against ADR-0004's own "no undiscoverable substitution" principle. **B-2** —
`LlmProviderFactory::create(&self, provider_name)` (`provider_factory.rs:356`) is env-only and
takes no model; `agent_host.rs` reads only `default_provider` (`:111-116`); the conversion
bridges in `crates/paladin-llm/src/config/bridge.rs` have no production caller (the only
`bridge::` hit outside the module is the unrelated `OrchestratorBridgeAdapter`). **B-3** —
`FallbackLlmAdapter` resends `request.clone()`, model included, to every hop (`fallback.rs:425`),
now wrapped by Phase 43's Cadence pacing keyed on `(provider, model)` — a chain step's own model
must feed that key too. **B-5** — `src/lib.rs:181-190` re-exports only the OpenAI, Anthropic and
DeepSeek adapter/config pairs. **G-2** — `CompatEngine::detect_empty_completion`
(`compat/engine.rs:651`) fires only on `finish_reason == length`. **G-1/G-11** — Kimi's single
window is `131_072` (`kimi/adapter.rs:196`) against 1,048,576 live for k3; DeepSeek declares
`64000` and defaults to `deepseek-chat`; `bridge.rs:96` still defaults Anthropic to
`claude-3-5-sonnet-20241022`. **G-7** — no `Refusal` variant exists in `llm_port.rs` or any
adapter. **H-1/H-2** — `conformance.rs` and `examples/live_vendor_smoke.rs` cover none of the
three originals. **Constraint** — `ProviderCapabilities` gaining a field is blocked by the
compile-time guard `provider_capabilities_gained_no_field` (`llm_port.rs:1977`, D-28 of Phase
26); the handoff's additive `ModelProfile` returned by a defaulted new `LlmPort` method is
non-breaking, and no such type exists on the tree yet. A clean break of `ProviderCapabilities`
instead is an explicit X-03 supersession event to be recorded as ADR-0051/0059/0060 were.
Precedent: the shared conformance suite (`crates/paladin-llm/src/conformance.rs`), the compat
preset engine, Phase 17-19's live temperature probes, and ADR-0004's ports-boundary validation.
Live checks (quota-error shapes, `json_object` under thinking, cache-read field names) run under
the existing credential gate with the env vars Paladin already reads.
**Success Criteria** (what must be TRUE):

  1. A provider-swap matrix test with mock transports runs the same agent definition with the
     provider switched only through config across all nine providers, asserting on the wire body
     and the outcome: each build-and-run either succeeds with equivalent behaviour or fails at
     build time with an error naming the provider, model and parameter. It covers manual and
     auto temperature on a `(1,1)` provider, the user prompt present on the wire, the per-step
     model in a fallback chain, and an empty `stop` completion — and it was red before the fixes.

  2. The OpenAI adapter sends the prompt text (`query`, with `context` appended when present) on
     the execution-service path, with a mock-transport regression test that fails on the current
     tree; and parameter support is modelled per provider and per model (`Range` / `Fixed` /
     `NotSent` for temperature, top_p, penalties, stop sequences) under one documented policy
     recorded as an ADR amending ADR-0004 that governs every adapter, the originals included — a
     Kimi swap with the default `auto_temperature` builds, an explicit out-of-range value is
     refused, and nothing is substituted undiscoverably.

  3. Provider construction honours `llm.<provider>` config blocks (model, base_url, timeout,
     max_retries, default max_tokens, context-window override) through a public library API
     routed through `config/bridge.rs`, per-agent `provider` + `model` resolve through the same
     path in `agent_host`, existing YAML files load unchanged (PROV-03), reasoning-capable
     defaults are 300 s not 60 s, and the facade exports all nine adapter/config pairs behind
     ADR-0046's features with `make api-surface-update` and a CHANGELOG entry.

  4. Per-model limits (context window, max output, reasoning support, fixed sampling values,
     JSON-mode support) come from a dated registry with config overrides; Commissary and
     `resolve_context_window` read it, so `kimi-k3` is rationed against 1,048,576 tokens and
     the stale Anthropic/DeepSeek defaults and model lists are replaced with dated entries.

  5. Every adapter maps empty output (`EmptyCompletion` on any stop reason, OpenAI included),
     refusal (a typed `LlmError::Refusal`, `Permanent`), usage cap (live-verified per compat
     vendor), cache read/write and reasoning tokens the same way, surfaces reasoning text on the
     normal and streaming paths, and puts stop sequences on the wire where the vendor accepts
     them; fallback chains send each hop its own model, key Cadence on it, and record the served
     model beside `SERVED_BY_METADATA_KEY`.

  6. All nine adapters are in the shared conformance suite (extended with capability
     truthfulness, `response_format` wire shape, empty completion, refusal, usage fields and the
     parameter-drop policy) and the credential-gated live smoke; the mdBook has a "Switching
     providers" page with the per-provider/per-model behaviour table; MIGRATION/CHANGELOG carry
     every intentional break; `make clean-code`, `make check-gates`, `make api-surface`,
     `make security` and the 82 % coverage floor stay green, with no `unwrap`/`expect`/`panic!`
     added to library code.
**Plans**: 32 plans (18 executed + 14 gap closure)

Plans:
**Wave 1**

- [x] 50-01-PLAN.md — Tracer: one agent from an `llm.openai` block through PaladinBuilder and PaladinExecutionService to a mock transport, red then fixed (B-4, PARITY-08); shared `user_turn_text` on all five request builders; the nine-provider provider-swap matrix with red cases ignored per fixing plan and the red run recorded
- [x] 50-02-PLAN.md — ADR-0061 amending ADR-0004 (refuse explicit, omit auto visibly; D-01..D-04a), PROMOTION next free 0062; `ModelProfile`/`ParamSupport`/`StopSupport`/`SamplingParam` and defaulted `LlmPort::model_profile` in paladin-ports; Pricing/Cadence/Fallback forward it
- [x] 50-04-PLAN.md — Nine-provider credential-gated live smoke with `--probe` (sampling, stop, output cap, JSON-object and JSON-schema probes) and `--dry-run`; autonomous harness only (the operator run is 50-17)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 50-03-PLAN.md — Shared `sampling::apply` (plus `apply_response_format` and `check_max_tokens`) and the model registry core (seed > override > visible fallback); CompatEngine and the five compat presets on them with stop on the wire; PaladinBuilder profile-first (explicit refused by name, auto omitted); Kimi matrix rows green

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 50-05-PLAN.md — OpenAI, DeepSeek, Anthropic and Gemini on the shared filter (no fabricated sampling values, no validate-then-discard, stop sequences sent, JSON mode and max output from the profile); nine-provider equivalence row green; M-B row for the whole policy
- [x] 50-17-PLAN.md — Operator runs the 50-04 probe on real keys (human-action checkpoint, blocks only 50-06) and answers owner reading R-2 per compat vendor (usage cap verified or accepted unverified); scrubbed dated `50-SEED-PROBE.md`

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 50-06-PLAN.md — Dated per-vendor registry tables seeded only from `50-SEED-PROBE.md` (D-05/D-08, after 50-17); adapters read them; stale Anthropic/DeepSeek defaults, Anthropic model list and adapter windows replaced

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 50-07-PLAN.md — `effective_capabilities`, `Commissary::for_model` and the HistoryTrimmer ration against the per-model window (kimi-k3 at its real window)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 50-08-PLAN.md — Owner checkpoint on the `*Config` shape (non_exhaustive + builders vs plain fields); `max_tokens`, `max_retries`, `context_window` (beats seeded windows), `model_overrides` on all nine configs, each read by its adapter; 300 s reasoning default on the direct constructors; every workspace literal (bridges, examples, root tests) migrated in the same commits; redacting `Debug`

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 50-09-PLAN.md — `LlmProviderConfig` keys and per-model overrides (existing YAML unchanged), D-13 layering with `${VAR}` placeholders and the 300 s reasoning default, per-provider `from_layers`, `LlmProviderFactory::create_from_config` and `is_configured`

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 50-10-PLAN.md — agent_host and facade_provisioner on `create_from_config` (per-agent provider/model, block `default_temperature` explicit); facade re-exports of all nine pairs (ADR-0046 defaults kept); matrix through the config API; `paladin-llm` breaks registered from a measured run

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 50-11-PLAN.md — Typed `LlmResponse.reasoning` and `StreamingResponse.reasoning`, `LlmResponse::new`; atomic migration of every literal; Phase 38 9.2 row extended

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 50-12-PLAN.md — `LlmError::Refusal` (Permanent) in all four tables; shared `classify_completion` (refusal > content filter > empty on any stop, buffered and streamed); compat, OpenAI and DeepSeek on it with cache-write and reasoning text

**Wave 11** *(blocked on Wave 10 completion)*

- [x] 50-13-PLAN.md — Anthropic refusal before empty, named stop reasons, thinking text and the stream's real stop reason; Gemini on the guard; empty streams typed; usage-cap overrides only where the probe evidences them, every compat vendor listed verified/unverified per R-2; empty-stop matrix green on all nine

**Wave 12** *(blocked on Wave 11 completion)*

- [x] 50-14-PLAN.md — `FallbackStep`/`from_steps`, per-hop model rewrite feeding Cadence, `paladin.served_model`, `hop_on_refusal` opt-in; `agent_runtime.model_fallback.steps`; matrix fully green; root breaks registered

**Wave 13** *(blocked on Wave 12 completion)*

- [x] 50-15-PLAN.md — Anthropic joins the shared conformance suite; seven new cases (capability truthfulness, response_format, empty completion buffered and streamed, refusal, usage fields, parameter-drop policy); `CASE_COUNT` 17

**Wave 14** *(blocked on Wave 13 completion)*

- [x] 50-18-PLAN.md — Owner reading R-1 (decision checkpoint): what D-01/D-07's "run-metadata entry" is; option-b carries the `paladin.*` keys onto `PaladinResult.run_metadata` and the streaming terminal chunk (registered break, literals migrated); ADR-0061 records the answer

**Wave 15** *(blocked on Wave 14 completion)*

- [x] 50-16-PLAN.md — mdBook "Switching providers" page with the per-provider/per-model table and a registry drift test; MIGRATION 9.8 consumer rows, changelogs, muster pins; closeout register check; ADR-0061 conforms; full phase gate

**Gap closure** *(from `50-UAT.md` test 5, owner decisions of 2026-10-17 on WINDOWS rows 74-76: G-50-5a major, G-50-5b major, G-50-5c minor)*

**Wave 16** *(gap-closure wave 1; blocked on Wave 15 completion)*

- [x] 50-19-PLAN.md — G-50-5b (row 75, option (a)): the OpenAI adapter always sends `max_completion_tokens` (chat and vision wire keys renamed, config names unchanged); gpt-5.4-nano body test on all three request paths; smoke drives gpt-5.4-nano; only gpt-5.4-nano and gpt-4o-mini recorded as measured

**Wave 17** *(gap-closure wave 2; blocked on Wave 16 completion)*

- [x] 50-20-PLAN.md — G-50-5c (row 76, option (a)): the Kimi adapter-wide fallback declares temperature, top_p and both penalties `NotSent`, matching the seeded kimi-k2.6/kimi-k3 rows; fallback-vs-seed test; temperature-range invariant covers Kimi

**Wave 18** *(gap-closure wave 3; blocked on Wave 17 completion)*

- [x] 50-21-PLAN.md — G-50-5a (row 74): tracer Grok wire -> compat mapper -> PricingLlmAdapter billed in full; per-preset usage semantics (Grok reasoning beside completion); vendor total parsed and reconciled under `paladin.usage_unreconciled`, D-02 kept
- [x] 50-22-PLAN.md — G-50-5a: `live_vendor_smoke --usage` and the operator's recorded usage-containment probe (`50-USAGE-PROBE.md`; human-action checkpoint; only 50-24 consumes it)

**Wave 19** *(gap-closure wave 4; blocked on Wave 18 completion)*

- [x] 50-23-PLAN.md — G-50-5a: Gemini tool-use prompt tokens folded; vendor totals reconciled on OpenAI, DeepSeek and Gemini (Anthropic containment, vision cache counts); disagreement conformance case on all nine (`CASE_COUNT` 19)

**Wave 20** *(gap-closure wave 5; blocked on Wave 19 completion)*

- [x] 50-24-PLAN.md — G-50-5a: the 50-22 live verdicts applied to each provider's usage semantics (red-first flips, mixed results ledgered open); usage mapping documented for all nine; MIGRATION 9.1 row

**Wave 21** *(gap-closure wave 6; blocked on Wave 20 completion)*

- [x] 50-25-PLAN.md — G-50-5a: additive `LlmError::empty_completion` / `LlmError::refusal` constructors; seven consumer files moved to shape-proof forms (no behaviour change)

**Wave 22** *(gap-closure wave 7; blocked on Wave 21 completion)*

- [x] 50-26-PLAN.md — G-50-5a: `BilledUsage`; `EmptyCompletion`/`Refusal` as `#[non_exhaustive]` struct variants with `billed_usage()`; compat buffered path bills; `LlmError` break measured and registered in the same commit

**Wave 23** *(gap-closure wave 8; blocked on Wave 22 completion)*

- [x] 50-27-PLAN.md — G-50-5a: every adapter bills empty and refused calls on both paths; PricingLlmAdapter prices them buffered and streamed; conformance case on all nine (`CASE_COUNT` 20)
- [x] 50-28-PLAN.md — G-50-5a: billed failures folded and settled to the treasury ledger in the agent loop, buffered and streamed (private `CallFailure`, no public type change)

**Wave 24** *(gap-closure wave 9; blocked on Wave 23 completion)*

- [x] 50-29-PLAN.md — G-50-5a remainder (b): `IncurredSpend` and the `ATTEMPT_SPEND` attempt spend scope; failed, timed-out and structured Battalion attempts count every billed call; the execution service records every call

**Wave 25** *(gap-closure wave 10; blocked on Wave 24 completion)*

- [x] 50-30-PLAN.md — G-50-5a remainder (a): `HOPPED_BILLS`; FallbackLlmAdapter records billed hops it moves past, PricingLlmAdapter prices them, the agent loop settles each call's parts in one row with a per-model breakdown

**Wave 26** *(gap-closure wave 11; blocked on Wave 25 completion)*

- [x] 50-31-PLAN.md — G-50-5a: end-to-end matrix case through real adapters to the ledger; billing contract documented (docs, changelogs, MIGRATION, ADR-0061); WINDOWS row 74 fixed; remainder (c) Grok cost cross-check kept open as the owner's accepted deferral

**Wave 27** *(gap-closure wave 12; blocked on Wave 26 completion)*

- [x] 50-32-PLAN.md — review WR-03: a hopped-past fallback bill is priced (pricer installed by `PricingLlmAdapter` before it delegates) and counted into `ATTEMPT_SPEND` at its hop, so an attempt cut off mid-call by the engine's deadline or idle bound is charged for it; `HoppedBills::install_pricer` / `count_and_take`, exactly-once fold; paused-clock drop tests (buffered, streamed, real engine); spend_scope docs and MIGRATION DR-18 limit text replaced; WR-03 WINDOWS row opened and closed

### Phase 50.1: Consumer-Surface Readiness for v0.11.0 (INSERTED)

**Goal**: A library consumer that moves onto `v0.11.0` can follow every recipe in `MIGRATION.md`
§9.8 "Library consumers" through the `paladin` facade alone, construct any of the nine `*Config`
types with one `new(..)` shape and set every knob (the timeout included) through a `with_*`
builder, and meet no silent behaviour change the guide does not name. web3sec measured the bump
ahead of time against the Phase 50 tip `21426037` (handoff Appendix A): 112 compile errors in 3
of its 12 crates, every one mapping to a registered row, fixed mechanically, suite back at its
2,592 / 0 / 2 baseline; re-measured at the sealed `a1128fc1`, DR-17 adds 35 more (147 in total,
workspace clean once fixed) and Phase 50's later plans add no other consumer break. The same exercise found six gaps that cost every consumer: the Aegis
family (`Aegis`, `ErrorHandlerSpec`, `TimeoutPolicy`, `StateDelta`, `NodeError`,
`NodeErrorSource`, `TimeoutKind`, `NodeId`, `Transience`) is not nameable through the facade, so
DR-1 / DR-2 / DR-8 / DR-9 force a direct `paladin-ai-core` dependency (28 of the 112 errors plus
the Aegis construction sites); the nine `*Config::new(..)` signatures still differ per provider
and `with_timeout_seconds` exists only on `LlmProviderConfig`, so a `#[non_exhaustive]` config's
timeout is set by assigning a public field; the consumer-surface muster pins almost none of what
actually broke; §9.8 omits the pre-`v0.10.0` §9.2 rows (31 of the 112 errors), the loss of
in-adapter 429 retry since Phase 43 (a direct-adapter consumer has **no pacing** unless it
composes Cadence), the decorator `model_profile` forwarding duty and the adapter-wide context
window following the seeded default model (Anthropic 200,000 → 1,000,000, DeepSeek 64,000 →
1,048,576, so `Commissary::from_port` rations 5× / 16× more); `compose_llm` needs a `PriceTable`
just to install Cadence; and the consumer's production models run on the unprofiled fallback. The
phase closes all six before the tag, where each is one additive export, one registered break, a
test, a docs row or a seed, rather than a v0.11.1 or a second consumer break afterwards.
**Depends on**: Phase 50 (sealed 2026-10-19: the nine `#[non_exhaustive]` `*Config` types and the
`config_knob_builders!` macro SURFACE-02 extends — PARITY-03; the dated seed tables and the
`50-SEED-PROBE.md` evidence SURFACE-06 reads — PARITY-04; the §9.8 rows DR-12..DR-18 SURFACE-04
extends) and Phase 49 (the muster and `make check-consumer-surface` SURFACE-03 extends — ADOPT-04).
**Sequenced after Phase 50 and before Phase 51** (owner decision 2026-10-18): SURFACE-02 changes
the `new(..)` shape of the nine types Phase 51 extends with a `reasoning_effort` field, so Phase
51 plans against the settled shape instead of migrating it twice; and it rides the single
coordinated v0.11.0 consumer bump (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`), so
web3sec meets every change here in the same move as DR-1..DR-18. The harmonisation todo of
2026-10-16 (`2026-10-16-harmonise-llm-config-new-signatures.md`), which Phase 50 deferred to
"Phase 46 if it can ride the same bump, otherwise a later API-hygiene phase", is resolved here by
owner decision; MIGRATION §9.8 DR-12's sentence that harmonisation is "not part of v0.11.0" is
amended by this phase.
**Requirements**: SURFACE-01, SURFACE-02, SURFACE-03, SURFACE-04, SURFACE-05, SURFACE-06
**UI hint**: no
**Source**: the fourth web3sec handoff of 2026-10-18, re-verified by its author 2026-10-19 after
the Phase 50 seal and filed verbatim in that final form as `50.1-HANDOFF-web3sec-consumer-surface.md`
— a pre-measurement of the consumer bump against the Phase 50 tip `21426037`, re-measured at
`a1128fc1`, with every `path:line` re-read there (zero drift from its first draft at `8488f11e`). Its §4 decisions **D-A** (whole `aegis` /
`node_error` modules or selected items on the facade), **D-B** (constructor shape:
`new(api_key, model)` + `with_base_url` / `with_timeout_seconds` / the existing knobs, or a shared
`ProviderConfig` builder; endpoint-only providers keep their endpoint argument), **D-C** (whether
`make check-consumer-surface` joins `check-gates` or CI) and **D-D** (SURFACE-05 / SURFACE-06 in
this phase or deferred) are settled at discuss, together with a fifth the filing raised, **D-E**
(below). The handoff's §5 shape — three plans: additive exports + pacing path; the registered
constructor break; docs + muster pins + seeds — is the starting point for planning, not a
constraint.
**Ground truth** (verified 2026-10-19 on `a1128fc1`, the Phase 50 seal; the handoff's
`8488f11e` line numbers still hold where re-read):
**SURFACE-01** — `src/core/platform/mod.rs` declares the local `pub mod container` with a
hand-picked list of `paladin_core::platform::container` modules and omits `aegis`, `node_error`,
`battlefield`, `waypoint` and `transience`; no other `pub use` under `src/` re-exports any of their
items. The types exist at `aegis.rs:43` (`Aegis`), `:243` (`TimeoutPolicy`), `:262`
(`ErrorHandlerSpec`), `battlefield.rs:222` (`StateDelta`), `node_error.rs:40` / `:72` / `:152`
(`TimeoutKind`, `NodeErrorSource`, `NodeError`), `transience.rs:32` (`Transience`) and
`waypoint.rs:295` (`NodeId`). **SURFACE-02** — `config_knob_builders!`
(`crates/paladin-llm/src/registry/mod.rs:158`) generates `with_max_tokens`, `with_max_retries`,
`with_context_window` and `with_model_overrides` only; `with_timeout_seconds` exists on
`LlmProviderConfig` alone (`config/llm.rs`); the nine `new(..)` signatures are as the todo tabulates.
**SURFACE-03** — of the twenty-one items in handoff Appendix B, `tests/consumer_surface_muster.rs`
mentions only `FinishReason` (as a value, never an exhaustive match) and `ArsenalError` (as a
return type); the other nineteen have zero hits. Neither `FinishReason`
(`crates/paladin-ports/src/output/llm_port.rs:1493`) nor `ArsenalError`
(`paladin-core/.../arsenal/core.rs:138`) is `#[non_exhaustive]`. `make check-consumer-surface` is
deliberately outside `check-gates` (Makefile: the gate members are one-second offline scripts, this
target compiles the facade) — but the Makefile also records that `make check`, `make lint`, and
CI's clippy, all-targets check and integration-test steps already compile or run the muster, which
bounds D-C to "is a dedicated CI step worth naming". **SURFACE-04** — §9.8 "Library consumers"
(`MIGRATION.md` from `## 9.8`) now carries DR-1..DR-18 (DR-18 was added by plans 50-29/50-30 after
the handoff's read); it has no mention of 429, Cadence, `model_profile` forwarding,
`max_context_tokens` or the P31 / P32 / P33 rows (its one pre-`v0.10.1` note is the
`TokenUsage::from_total` pitfall). The adapter window change is real: `anthropic/adapter.rs:1521`
and `:1561` assert `max_context_tokens == Some(1_000_000)` from the seed, `deepseek/adapter.rs:451`
follows the same `context_window` → seed → fallback chain, and the knob macro's own rustdoc states
that a 429 "is never retried here, because the Cadence decorator owns waiting". **SURFACE-05** —
`src/infrastructure/cadence.rs:170` `compose_llm(llm, &price_table, cadence)` is the only facade
composition path; `paladin_llm::cadence::with_cadence` is not re-exported anywhere under `src/`.
**SURFACE-06 — the ask conflicts with the seed rule as written, hence D-E.**
`registry/seeded.rs` states "a model is listed only when it was measured end to end" and "a value
with no evidence in that transcript is not written here", and its DeepSeek table says in so many
words that `deepseek-v4-pro` "is listed but was not probed, so it is not seeded"; the Anthropic
table likewise seeds only the one probed model and says the other listed ids "were not probed, so
their sampling behaviour is unknown and they are not listed". `50-SEED-PROBE.md` holds listing
rows for `claude-opus-4-8` (`max_input_tokens=1000000`, `max_tokens=128000`, effort
`low..max`+`xhigh`) and `deepseek-v4-pro` (`context_window=1048576`, `max_output_tokens=393216`,
effort `low/high/max`) but no sampling, stop or JSON probe for either. Seeding them from the
listing alone would write a half-measured entry, which the rule forbids because a seeded entry
*replaces* the fallback and every unmeasured field becomes a refusal. **D-E**, for the operator:
(a) re-run the 50-04 probe on the operator's keys for exactly these two models (a 50-17-style
human-action checkpoint) and seed from the new transcript; (b) extend the registry with a
listing-only seed tier that carries window / output / reasoning facts and leaves the sampling
fields at their fail-closed values, documented as such in ADR-0061; or (c) decline SURFACE-06 and
let the consumer set `model_overrides` for its two models (recipe added to §9.8). The filing
recommends (a) if a key run is cheap, otherwise (c); (b) changes a Phase 50 decision (D-05/D-08)
and would need its own ADR note. `deepseek-v4-flash` is not seedable under any option (absent from
DeepSeek's own listing; only `deepseek-v4-flash-0731` appears, in the Qwen listing), as the
handoff itself says.
**Success Criteria** (what must be TRUE):

  1. The DR-1, DR-2, DR-8 and DR-9 recipes in MIGRATION §9.8 compile using only `paladin::`
     paths; the muster pins a `with_aegis(Aegis { on_error: Some(ErrorHandlerSpec::Absorb { .. }),
     timeout: Some(TimeoutPolicy { .. }), .. })` construction and a read of
     `node_errors[i].{node_id, attempt, transience, source}` through the facade; the four §9.8 rows
     name the import path; `make api-surface-update` and the CHANGELOG carry the additive exports.

  2. All nine `*Config` types construct with the same `new(..)` shape (endpoint-only providers
     excepted per the todo), every knob including the timeout is settable through a `with_*`
     builder, `from_env()` is unchanged, the break is registered in
     `.cargo/semver-checks-allowlist.toml`, MIGRATION §9.2 and a §9.8 DR row with DR-12's deferral
     sentence amended, `make api-surface-update` is refreshed, every in-tree call site (examples,
     `config::bridge`, the provider factory, tests) is migrated in the same commits, and the
     `paladin-llm` doctests show the new shape.

  3. Every item in handoff Appendix B is pinned in `tests/consumer_surface_muster.rs`, at least one
     new pin was proven red before it went green (recorded as `50.1-MUSTER-RED-EVIDENCE.md`, the
     Phase 49 pattern), the muster compiles at the seal, the `FinishReason` / `ArsenalError`
     exhaustiveness question is answered one way (exhaustive match pinned, or `#[non_exhaustive]`
     registered in §9.2), and D-C is recorded.

  4. MIGRATION §9.8 "Library consumers" carries the pre-`v0.10.0` pointer list (P31, P32, P33), the
     429 / Cadence behaviour row with the `build_cadence` / `compose_llm` (or the SURFACE-05 path)
     recipe and the spend-cap `UsageLimitExceeded` note, the decorator `model_profile` forwarding
     line, and the adapter-wide context-window consequence with its two remedies (`context_window`
     or `Commissary::for_model`).

  5. One doc test wraps an `Arc<dyn LlmPort>` in Cadence through a `paladin::` path with no
     `PriceTable` (or D-D records the deferral).

  6. `claude-opus-4-8` and `deepseek-v4-pro` resolve to seeded profiles with no
     `paladin.unprofiled_model` for them, under the D-E option the operator chooses, with the seed
     rule in `registry/seeded.rs` and ADR-0061 still true of the tree (or D-D / D-E record the
     deferral and the §9.8 `model_overrides` recipe stands in).
**Plans**: 9 plans in 7 waves (planned 2026-10-19; plans 02, 06 and 07 are `autonomous: false`)

Plans:
**Wave 1**

- [x] 50.1-01-PLAN.md — Tracer: the Aegis family through the facade (five whole-module re-exports in `src/core/platform/mod.rs` plus nine prelude names, D-01/D-02) with the D-04 muster pins proven red first (`50.1-MUSTER-RED-EVIDENCE.md`, D-12); the `with_cadence`/`CadenceWiring` re-export with a facade-only doc test (SURFACE-05, D-13); the four §9.8 D-19 additions (Phase 31/32/33 rows cited by type); the 17 in-tree examples moved to `paladin::` paths (D-03)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 50.1-02-PLAN.md — `checkpoint:decision` on the one-way D-05 door (plus the D-06/C1 choice), then the tracer for the registered break: `with_base_url` and `with_timeout_seconds` macro arms for all nine types, `DeepSeekConfig::new(api_key, model)` with every call site and its muster pin, semver measured before any allowlist text, §9.2 row, new §9.8 DR-19, DR-12 amended

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 50.1-03-PLAN.md — Exhaustive `FinishReason`/`ArsenalError` pins (D-11) with a scripted-mutation red proof, the named `consumer-surface` CI job (D-10), the remaining Appendix B pins 3, 4, 6-12
- [x] 50.1-04-PLAN.md — Gemini (tracer), then Qwen + Grok, then Kimi move to `new(api_key, model)`; each commit carries its type's code, call sites, measured register entry and §9.2 row

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 50.1-05-PLAN.md — `OpenAIConfig.model` added without a break (stored, validated, `Debug`-printed; `declared_capabilities` not rewired per research C1) plus `with_organization`; then `OpenAIConfig::new(api_key, model)` with every call site and its register entry

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 50.1-06-PLAN.md — `checkpoint:decision` on the D-07 `max_tokens` source (new vs `from_env` asymmetry), the Anthropic tracer, then the SURFACE-02 seal: final semver measurement, DR-19 complete, §9.8 muster paragraph, harmonise todo moved to `completed/` with `resolves_phase: 50.1`
- [x] 50.1-07-PLAN.md — `autonomous: false`: the D-16 candidate id table (D-15 rule over the `50-SEED-PROBE.md` listings) plus an offline dry run (tracer), the operator's human-action probe run on their keys, the scrubbed `50.1-SEED-PROBE.md` (depends on no code plan; placed here so the operator checkpoint does not hold waves 2-5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 50.1-08-PLAN.md — Seed `claude-opus-4-8` and `deepseek-v4-pro` from the new transcript only (tracer, every ripple closed), then the rest of the measured Anthropic and Gemini range (D-15, D-17 defaults unchanged)

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 50.1-09-PLAN.md — OpenAI, then Grok/Kimi/Qwen seeds; the §9.8 "Models seeded in Phase 50.1" bullet naming every behaviour a new seed brings; the conditional `model_overrides` recipe if a web3sec model failed the probe (success criterion 6 closes one way); the phase seal gates run once as plan-level verification

### Phase 51: Adjustable Reasoning Effort

**Goal**: One provider-neutral, per-agent (and per-request) **reasoning-effort** setting that
every adapter translates exactly to its vendor's native form, or refuses loudly before the HTTP
call. Today Paladin has no way to control how hard a model reasons: no request struct in any of
the nine adapters serialises an effort, thinking or budget field, reasoning is handled only on the
response side (token counts, empty-output detection, parsed-then-discarded reasoning text), and
the vendor default always applies. For Moonshot `kimi-k3` that default is `max` — measured live,
a one-line question spent the whole 16,000-token output budget on reasoning in about eight minutes
and returned no answer, where `low` answered in 56 s and `disabled` in 2 s (handoff §4.1). The
central hazard is the **silent no-op**: at least four vendors return HTTP 200 while ignoring or
coercing the requested effort, and `kimi-k3` ignores an OpenAI-style top-level `reasoning_effort`
outright — only `thinking.effort` works (§5, Appendix A). So the design is a `#[non_exhaustive]`
`ReasoningEffort` enum on `LlmRequest` (the `response_format` precedent, D-28), a per-Paladin
default on `PaladinData` with a builder setter and a YAML agent field, a per-provider default
through Phase 50's config-driven construction, a compile-enforced per-preset wire declaration in
the compat engine, model-aware mapping in the native adapters, a new additive
`LlmError::UnsupportedParameter` for anything that cannot be mapped losslessly, requested and
effective effort on the response and the trace stream, and a live conformance check per vendor
proving the effort field is actually parsed. Effort, output budget and timeout are designed
together: raising effort without raising `max_tokens` turns into `EmptyCompletion`. web3sec needs
this at **request level** (it builds adapters directly and calls `LlmPort::generate`), and wants
per-role settings for cost/latency control and A/B benchmarking.
**Depends on**: Phase 50 (the ADR amending 0004 and the per-provider/per-model parameter-support
policy this phase extends — PARITY-02; the `llm.<provider>` config-driven construction and the
300 s reasoning-capable defaults the per-provider effort default rides on — PARITY-03; the dated
per-model profile registry that holds each model's supported levels and budget bounds — PARITY-04;
the reasoning-text and reasoning-token surfaces — PARITY-05; and per-hop models in fallback chains
so each step resolves effort against its own model — PARITY-06; handoff §9). **Also Phase 50.1** (inserted 2026-10-19): SURFACE-02 settles the one `new(..)` shape of the nine `*Config` types this phase extends with a `reasoning_effort` field, so plan against the 50.1 shapes; nothing in 50.1 touches `LlmRequest`, `ReasoningEffort` or `LlmError::UnsupportedParameter`. The handoff's
"sooner" option — Plans 1-3 against adapter-wide supported sets, migrated to model profiles later —
is declined: it costs a second pass over every preset. **Sequenced ahead of Phases 46 and 47**
(owner decision 2026-10-14): the third consumer-driven phase that must land in **v0.11.0**,
because the ADR, the `ReasoningEffort` public type, the MIGRATION §9.2 rows and the mdBook
"Reasoning effort" page change what Phase 46's docs pass and migration guide must say and what
Phase 47 tags.
**Requirements**: REASON-01, REASON-02, REASON-03, REASON-04, REASON-05, REASON-06, REASON-07
**UI hint**: no
**Source**: the third web3sec handoff of 2026-10-13, filed verbatim as
`51-HANDOFF-web3sec-reasoning-effort.md` — a read-only audit of `origin/main` @ `df24689c`
(post-`v0.10.1`) with live Moonshot measurements dated 2026-10-07 and 2026-10-13 (`kimi-k3`
`valid_efforts: [low, high, max]`, `default_effort: max`; `thinking:{type:"disabled"}` accepted
despite `supports_thinking_type: "only"`; top-level `reasoning_effort: "bogus"` → HTTP 200,
ignored; `thinking.effort: "bogus"` → HTTP 400) and desk research on the other seven vendors, every
row of which is to be re-verified against its cited source and a live probe before it is encoded
into a model profile. The handoff proposed "Phase 39 if parity is 38" against its own roadmap
snapshot; it is Phase 51 here, and it ships in v0.11.0, not as a patch on the v0.10.1 the handoff
was read against. Its §7 decisions **D-1** (enum shape — keep `Auto` distinct from `None` and
include `Budget(u32)` now, because `Auto` is needed where "on" must be stated explicitly and
`Budget` is the only control Qwen and Gemini 2.5 offer), **D-2** (placement — `LlmRequest` plus
a per-agent default on `PaladinData`; never `PromptParameters`, whose hand-written
`PartialEq`/`Ord`/`Hash` and twelve struct literals would silently exclude or break on a new
field; never `ProviderCapabilities`, guarded against gaining a field), **D-3** (fail loudly by
default on any value that cannot be mapped losslessly, with nearest-level substitution as an
explicit opt-in — recorded as an ADR amending 0004, consistent with Phase 50's D-1), **D-4**
(reasoning text typed on `LlmResponse` and the streaming delta, not buried in `metadata`) and
**D-5** (Anthropic uses `output_config.effort` where the model supports it and
`thinking.budget_tokens` only on models that still accept it, chosen per model profile; the
deprecated form is never sent to a model that rejects it) are settled at discuss. The handoff's §8
shape — one phase, about five plans in the order red tests → types and plumbing → compat presets →
native adapters → live conformance, docs and ADR — is the starting point for planning, not a
constraint. Its §10 out-of-scope list holds: no native tool calling or interleaved thinking with
tools (ADR-0042), no Anthropic per-message effort beta, no live discovery of supported levels from
vendor `/models` endpoints (a possible follow-up to Phase 50's D-3), and no web3sec-side changes.
**Ground truth** (verified 2026-10-14 on `885db3b8`; the handoff's line numbers have drifted and
the execution service now lives under `src/application/services/`, but every claim re-checked
holds): **nothing request-side** — a grep for `reasoning_effort|budget_tokens|thinkingConfig|
thinkingBudget|enable_thinking|output_config|"think"` over `crates/`, `src/` and `tests/` hits
only the unrelated RAG budget helper (`rag_retrieval_service.rs:153`), never an adapter request
struct. **Placement holds** — `LlmRequest` is `#[non_exhaustive]` (`llm_port.rs:763`) with the
additive `response_format` field (`:784`) and `with_response_format` (`:910`), threaded from the
run context at `paladin_execution_service.rs:2817-2820`; the other two request-construction sites
are `:3003` and `:3586`; `LlmError` is `#[non_exhaustive]` (`:289`); `PaladinData` already adds
per-agent knobs as `#[serde(default)]` fields (`paladin.rs:175-198`); `AgentDefinition`
(`src/config/agents.rs:321`) carries `model` and `provider` but no effort; the
`ProviderCapabilities` guard `provider_capabilities_gained_no_field` sits at `llm_port.rs:1977`.
**Compat engine holds** — `CompatRequestParameters` (`compat/engine.rs:129-156`) is five `bool`s
with deliberately no `Default` ("a new preset must be a compile error until its author states a
position for every field", `:122-126`); Kimi's position is at `kimi/adapter.rs:245-251`;
`build_request` (`:472`) reports dropped parameters only through `log::debug!` (`:502`), the path
this phase must **not** reuse for effort; `detect_empty_completion` (`:651`) fires only on
`length`. **Response side holds** — `reasoning_content` is deserialised in `compat/types.rs:80`
and `deepseek/adapter.rs:202` and never reaches callers; `TokenUsage.reasoning_tokens`
(`token_usage.rs:44`, `with_reasoning` `:114`) is filled by OpenAI (`:533`), Anthropic
`thinking_tokens` (`:835`), Gemini `thoughtsTokenCount` (`:1143`), DeepSeek and the compat
engine; Anthropic's `detect_no_text_content` (`:957`) still turns thinking-only responses into
`EmptyCompletion`. **Kimi's default timeout is 60 s** (`kimi/adapter.rs:118`, `:139`) against a
measured 56 s for `low` — Phase 50's 300 s reasoning-capable default is a prerequisite, not a
nicety. **No `ModelProfile` type exists on the tree** — the per-model supported-level set this
phase reads (REASON-04) is created by Phase 50 (PARITY-04). `conformance.rs` and
`examples/live_vendor_smoke.rs` exist and are the homes for the offline wire-body cases and the
live invalid-effort probes. Precedent: the `response_format` threading (Phase 26, D-27/D-28),
`ReasoningAgentOptions` (`src/presets/mod.rs:62`) as the option-type pattern, ADR-0004's
"no undiscoverable substitution" principle (`:92`, `:164`, `:179`), and the Phase 17-19 live
temperature probes.
**Success Criteria** (what must be TRUE):

  1. A `#[non_exhaustive]` `ReasoningEffort` (`Off`, `Auto`, `Minimal`, `Low`, `Medium`,
     `High`, `XHigh`, `Max`, `Budget(u32)`) is settable per request (`LlmRequest.reasoning`

     + `with_reasoning`), per agent (`PaladinData`, a `PaladinBuilder` setter, a YAML
     `AgentDefinition` field) and per provider (`llm.<provider>.reasoning_effort`), resolved
     request → agent → provider config → vendor default on every `LlmRequest` the execution
     service builds — and an unset value produces a byte-identical request body on every
     adapter, pinned by a mockito wire-body test per adapter/preset that was red first.

  2. Every adapter and preset declares its reasoning wire form — a non-`bool` `ReasoningWire`
     on `CompatRequestParameters` the compiler forces each preset author to state (generic
     OpenAI-compatible defaults to `Unsupported`), and a private model-aware `map_reasoning`
     in Anthropic, OpenAI, DeepSeek and Gemini — and maps each level exactly or returns the new
     additive `LlmError::UnsupportedParameter { param, value, provider, model }`, classified
     `Permanent`, before any HTTP call: a level the vendor lacks, a level gated off for that
     model, `Off` on an always-thinking model, `Budget` where only levels exist, and a level
     the vendor is known to coerce silently all refuse; nearest-level substitution is an
     explicit opt-in, off by default, that logs `warn!` and records both efforts.

  3. No preset emits a field its vendor ignores: a silent-no-op regression test asserts the
     Kimi preset never emits top-level `reasoning_effort` and sends `thinking:{type, effort}`
     instead, and the credential-gated live smoke proves per vendor, where a key exists, that
     an invalid effort is rejected (the field is parsed) and that a higher level reports more
     `reasoning_tokens`.

  4. Supported levels and budget bounds are per model, read from Phase 50's dated model-profile
     registry with config overrides rather than adapter-wide constants; each hop of a fallback
     chain resolves effort against its own model; and `Budget(n)` is validated against the
     vendor's bounds and the request's `max_tokens` before send, with `EmptyCompletion` kept as
     the runtime backstop.

  5. Requested and effective effort are recorded in `LlmResponse.metadata` beside
     `SERVED_BY_METADATA_KEY` and emitted on the Phase 28 trace stream; every adapter fills
     `TokenUsage.reasoning_tokens` when the vendor reports it; and reasoning text reaches
     callers through the typed shape settled at discuss (D-4) on both the normal and the
     streaming path, building on Phase 50's surface rather than duplicating it.

  6. An ADR amending ADR-0004 records the fail-loud policy and its opt-in fallback; the mdBook
     has a "Reasoning effort" page with the per-provider/per-model mapping table and the
     effort/output-budget/timeout guidance; MIGRATION §9.2 and CHANGELOG carry the additive
     type, field, variant and config key with `make api-surface-update`; and `make clean-code`,
     `make check-gates`, `make api-surface`, `make security`, the 82 % coverage floor and the
     doc tests stay green, with no `unwrap`/`expect`/`panic!` added to library code.
**Plans**: TBD

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

*Extended: 2026-09-24 — **v0.11.0 "Treasurer Spend Governance" roadmap created.** Phases 38-47
added under a new milestone label, forward work sourced from `.project/Milestone_14-Treasurer/`
(overview + Epic 1 PRD `prd-treasurer-spend-governance.md`, R1-R6) plus the supporting scope in
PROJECT.md *Current Milestone* and `.planning/research/SUMMARY.md` (2026-09-24) — the same
operator-instructed, not ingest-derived pattern as Phases 17-37.1. The PRD's open operator question
is answered: full governance (per-tenant + per-API-key), operator-confirmed 2026-09-24. Seven new
requirement ID prefixes — the thirty-second through thirty-eighth, recycling none of the
thirty-one spent: `PRICE-*` (3), `LEDGR-*` (4), `TENANT-*` (2), `ALLOW-*` (5), `PACE-*` (5),
`LEGACY-*` (4), `STORE-*` (3) — 26 requirements. `PLAT-*` (07-09), `OBS-*` (05), `CURR-*` (22-25)
and `SHIP-*` (07) are extended in place, not new prefixes — 9 requirements. 35 v1 requirements
total, mapped 1:1 to 10 phases with 100% coverage validated. Phase order follows the research
summary's dependency chain: 38 (pricing/cost, plus the two gating design decisions — the mid-run
enforcement attachment point across `WarEngine` and `PaladinExecutionService`, and derive-on-read
vs running-balance ledger model) is the keystone; 39 (ledger) depends on 38; 40 (tenant identity +
PLAT-07) is sequenced before enforcement but has no content dependency on 39; 41 (admission
enforcement) depends on 39+40; 42 (mid-run halt + PLAT-09) depends on 38's design decision and 41's
Treasurer facade; 43 (rate pacing) is parallel-schedulable, depending only on 38's design seams; 44
(legacy clean-break) and 45 (RustFS + platform/observability) are sequenced after the Treasurer
feature phases to avoid diff conflicts; 46 (docs/hygiene) and 47 (release) close the milestone.
Phases 1-37.1 unchanged and unrenumbered; every `### Phase N:` header is verbatim.*
