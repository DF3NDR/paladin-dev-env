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
| **Treasurer Spend Governance** | 38-47 | 🚧 **In progress** | Forward work — not ingest-derived. Sourced from `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD `prd-treasurer-spend-governance.md`, R1-R6), the supporting scope in PROJECT.md *Current Milestone*, and `.planning/research/SUMMARY.md` (2026-09-24). Operator-confirmed full governance (per-tenant + per-API-key), 2026-09-24. |

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

**Treasurer Spend Governance (Phases 38-47)** — in progress, not yet shipped. Forward work sourced
from `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD R1-R6) and
`.planning/research/SUMMARY.md`; requirements minted in `.planning/REQUIREMENTS.md`.

- [ ] **Phase 38: Design Seams & Pricing/Cost Producer** - Record the mid-run enforcement attachment point and the ledger balance model, then ship an operator-configured price table and a fixed-point cost function that reaches `ExecutionMetadata.cost_estimate`
- [ ] **Phase 39: Spend Ledger** - A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters behind one contract suite, reserve-then-settle draws, idempotent settlement, and queryable spend
- [ ] **Phase 40: Tenant Identity & Run-Read Scoping** - Map API keys to tenants on the `Principal`, attribute every run to its submitting principal, and scope every run-read route to the calling principal
- [ ] **Phase 41: Admission-Time Allowance Enforcement** - Rolling-period allowances with an optional lifetime cap, refused at admission when exhausted, with a non-blocking warn-threshold notice
- [ ] **Phase 42: Mid-Run Halt & SSE Terminal Status** - A resumable, checkpoint-preserving halt when an in-flight run's next draw would overspend, on both the engine and agent-loop paths, plus the SSE `done`/`Cancelled` fix
- [ ] **Phase 43: Rate Pacing** - In-process and Redis-shared 429 back-off with jitter, a distributed cache-stampede lock, and safe degradation when Redis is unavailable
- [ ] **Phase 44: Legacy Clean-Break Removal** - Remove legacy Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts and `PaladinError::LlmError(String)`, migrating the last string-matching retry check to the typed taxonomy
- [ ] **Phase 45: RustFS Swap & Platform/Observability Deviations** - Replace MinIO with RustFS across dev/test and CI, wire SSE/webhook emission for legacy `Runnable::Agent` runs, and close the tracing-overhead gap
- [ ] **Phase 46: Docs Currency & Hygiene** - Close the v0.10.0 audit's docs fixes, publish the Treasurer mdBook page and migration guide, reconcile Nyquist validation, and dispose of the three v2 debt lines
- [ ] **Phase 47: v0.11.0 Crate Release** - Tag and publish v0.11.0: all 12 crates on crates.io, CHANGELOG and MIGRATION complete, publish-order gate green

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

- [ ] 38-03-PLAN.md — the `treasurer:` config section (exact decimal parsing, boot validation) and pricing installed on both production run paths (PRICE-01)
- [ ] 38-04-PLAN.md — `LlmResponse.cost` with every literal migrated and the break registered; non-streaming and fallback-hop pricing (PRICE-03)
- [ ] 38-05-PLAN.md — JSON herald currency field; table herald renders real metadata and cost (PRICE-03)
- [ ] 38-06-PLAN.md — `NodeFinished.cost`/`RunFinished.cost` carriers and `TraceDispatcher::total_cost` (PRICE-03)

**Wave 4** *(blocked on Wave 3)*

- [ ] 38-07-PLAN.md — `PaladinResult.cost`, agent-loop cost accumulation, and the engine bridge from each Paladin attempt to `NodeFinished.cost` (PRICE-03)

**Wave 5** *(blocked on Wave 4)*

- [ ] 38-08-PLAN.md — engine-path `ExecutionMetadata` producer: `from_run_finished`, `HeraldTraceSink`, `RunWorkerPool::with_herald` (PRICE-03)

**Wave 6** *(blocked on Wave 5)*

- [ ] 38-09-PLAN.md — closeout: `cargo semver-checks` measurement and registration, CHANGELOG, API-surface refresh, full gate suite (PRICE-01..03)

**Research flag**: yes — the engine-path-vs-agent-loop attachment point for mid-run `TokenBudget`
enforcement is, per research, the single largest open architectural question this milestone
surfaces; needs an ADR-level design pass before Phase 42 planning.

### Phase 39: Spend Ledger

**Goal**: A durable, race-proof spend ledger exists that every later Treasurer phase can draw
against and query.
**Depends on**: Phase 38 (cost function to persist; reserve/settle schema decision)
**Requirements**: LEDGR-01, LEDGR-02, LEDGR-03, LEDGR-04
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
**Plans**: TBD

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
**Plans**: TBD

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
**Plans**: TBD

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
**Plans**: TBD
**Research flag**: yes — mid-run halt has no prior art among surveyed comparable systems (none
have durable multi-step runs); the streaming settle-on-nonexistent-usage pitfall needs explicit
handling by reusing `TokenBudget`'s existing cutoff mechanism rather than inventing a second one.

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
**Plans**: TBD
**Research flag**: yes — no adjacent in-tree pattern to imitate for the stampede lock, and the
fail-open-vs-fail-closed decision on Redis unavailability is a genuine open design question the PRD
does not resolve; needs deeper phase-specific research.

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
**Plans**: TBD

### Phase 45: RustFS Swap & Platform/Observability Deviations

**Goal**: The dev/test/CI object store runs on a maintained, pinned image instead of a terminal
MinIO pin, and two standing accepted deviations (legacy-agent SSE/webhook silence, tracing
overhead) are closed or re-measured.
**Depends on**: Phase 44 (sequenced after legacy removal to avoid diff conflicts); functionally
independent of the Treasurer phases
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
**Plans**: TBD
**Research flag**: yes — RustFS's S3 API-surface parity (presigned URLs, multipart uploads, ETag
format) is MEDIUM-confidence only; the phase's first work item should close this via the existing
`FileStoragePort` contract-test suite before any adapter code is written.

### Phase 46: Docs Currency & Hygiene

**Goal**: The v0.10.0 audit's docs debt is closed, v0.11.0's own surface is documented, and the
milestone's validation/tech-debt bookkeeping is current.
**Depends on**: Phase 45 (documents everything the milestone shipped, so it runs last among
feature work)
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
**Depends on**: Phase 46 (needs the milestone's docs/hygiene current before release)
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
| 38. Design Seams & Pricing/Cost Producer | v0.11.0 | 0/9 | Planned | - |
| 39. Spend Ledger | v0.11.0 | 0/TBD | Not started | - |
| 40. Tenant Identity & Run-Read Scoping | v0.11.0 | 0/TBD | Not started | - |
| 41. Admission-Time Allowance Enforcement | v0.11.0 | 0/TBD | Not started | - |
| 42. Mid-Run Halt & SSE Terminal Status | v0.11.0 | 0/TBD | Not started | - |
| 43. Rate Pacing | v0.11.0 | 0/TBD | Not started | - |
| 44. Legacy Clean-Break Removal | v0.11.0 | 0/TBD | Not started | - |
| 45. RustFS Swap & Platform/Observability Deviations | v0.11.0 | 0/TBD | Not started | - |
| 46. Docs Currency & Hygiene | v0.11.0 | 0/TBD | Not started | - |
| 47. v0.11.0 Crate Release | v0.11.0 | 0/TBD | Not started | - |

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
