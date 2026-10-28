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
| **Treasurer Spend Governance** | 38-51 | ✅ **Shipped v0.11.0 (2026-10-27)** — [archive](milestones/v0.11.0-ROADMAP.md). Tag `v0.11.0` on merge commit `70657919` (2026-10-26, release run `40396108246`), all 12 publishable crates on crates.io at `0.11.0` — one tag, no patch needed; see MILESTONES.md. Phases 38-45 complete by 2026-10-12; Phases 48-51 (and the inserted 50.1) added 2026-10-12..19 and complete by 2026-10-23; Phases 46-47 complete 2026-10-24/26; all 15 phases closed 2026-10-27 | Forward work — not ingest-derived. Sourced from `.project/Milestone_14-Treasurer/` (overview + Epic 1 PRD `prd-treasurer-spend-governance.md`, R1-R6), the supporting scope in PROJECT.md *Current Milestone*, and `.planning/research/SUMMARY.md` (2026-09-24). Operator-confirmed full governance (per-tenant + per-API-key), 2026-09-24. Extended 2026-10-12 with Phase 48 (Base Entity Rename) and 2026-10-14 with Phase 49 (Downstream Adoption of v0.10.1, from the web3sec handoff), both sequenced before Phases 46-47; extended again 2026-10-14 with Phase 50 (Provider Parity & Configuration-Driven Model Swap, from the second web3sec handoff), also sequenced before Phases 46-47; extended again 2026-10-19 with Phase 50.1 (Consumer-Surface Readiness for v0.11.0, inserted from the fourth web3sec handoff), sequenced after Phase 50 and before Phase 51. |
| **Dogfood Cleanup** | 52-58 | 🚧 **In progress (v0.11.1, opened 2026-10-27)** — 7 phases, 53 requirements; a patch release unless a security fix forces `v0.12.0` (decision point in Phase 53) | Forward work — not ingest-derived. Sourced from `.project/v0.11.1/` (`01-carried-gaps-inventory.md` rows A1…G4, `02-dogfood-findings-log.md` entries DF-nn, `03-suggestions-and-scoping.md`) and PROJECT.md *Current Milestone*. Cleanup and bug-fix only: no new ports, adapters or HTTP routes except where a security fix needs one; stubs are fenced, not finished. Phase 56's detailed scope is filled from the findings log once the maintainer's runs and the web3sec return arrive. |

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

<details>
<summary>✅ <strong>v0.11.0 Treasurer Spend Governance (Phases 38-51)</strong> — SHIPPED 2026-10-27 · 174 plans, 63/66 requirements (ADOPT-01…03 superseded by the owner's 2026-10-14 scope cut) · released to crates.io as <code>0.11.0</code> on one tag</summary>

- [x] **Phase 38: Design Seams & Pricing/Cost Producer** - Record the mid-run enforcement attachment point and the ledger balance model, then ship an operator-configured price table and a fixed-point cost function that reaches `ExecutionMetadata.cost_estimate` (completed 2026-09-26)
- [x] **Phase 39: Spend Ledger** - A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters behind one contract suite, reserve-then-settle draws, idempotent settlement, and queryable spend (completed 2026-09-28)
- [x] **Phase 40: Tenant Identity & Run-Read Scoping** - Map API keys to tenants on the `Principal`, attribute every run to its submitting principal, and scope every run-read route to the calling principal (completed 2026-09-30)
- [x] **Phase 41: Admission-Time Allowance Enforcement** - Rolling-period allowances with an optional lifetime cap, refused at admission when exhausted, with a non-blocking warn-threshold notice (completed 2026-10-12)
- [x] **Phase 42: Mid-Run Halt & SSE Terminal Status** - A resumable, checkpoint-preserving halt when an in-flight run's next draw would overspend, on both the engine and agent-loop paths, plus the SSE `done`/`Cancelled` fix (completed 2026-10-07)
- [x] **Phase 43: Rate Pacing** - In-process and Redis-shared 429 back-off with jitter, a distributed cache-stampede lock, and safe degradation when Redis is unavailable (completed 2026-10-09)
- [x] **Phase 44: Legacy Clean-Break Removal** - Remove legacy Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts and `PaladinError::LlmError(String)`, migrating the last string-matching retry check to the typed taxonomy (completed 2026-10-11)
- [x] **Phase 45: RustFS Swap & Platform/Observability Deviations** - Replace MinIO with RustFS across dev/test and CI, wire SSE/webhook emission for legacy `Runnable::Agent` runs, and close the tracing-overhead gap (completed 2026-09-30)
- [x] **Phase 46: Docs Currency & Hygiene** - Close the v0.10.0 audit's docs fixes, publish the Treasurer mdBook page and migration guide, reconcile Nyquist validation, and dispose of the three v2 debt lines (completed 2026-10-24)
- [x] **Phase 47: v0.11.0 Crate Release** - Tag and publish v0.11.0: all 12 crates on crates.io, CHANGELOG and MIGRATION complete, publish-order gate green (completed 2026-10-26)
- [x] **Phase 48: Base Entity Rename** - Rename the base-layer entity wrapper `Node<T>` → `Entity<T>` (payload field `node` → `data`; clean break under a Phase-48-only X-03 supersession, or a one-release deprecated alias — decided at discuss; serde alias for persisted JSON), disposition the unused base-layer items, and reserve "node" for the graph sense only — **executes before Phases 46 and 47** (added 2026-10-12, after Phase 44 sealed) (completed 2026-10-14)
- [x] **Phase 49: Downstream Adoption of v0.10.1 (web3sec Consumer Bump)** - Scope cut to its Paladin-side half on 2026-10-14 (owner decision): the compile-only consumer-surface muster and the `MIGRATION.md` library-consumer note landed (plans 49-01..03); the consumer bump itself (ADOPT-01..03) is superseded by one web3sec move straight to `v0.11.0` after Phases 50 and 51 (todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md`) (completed 2026-10-14)
- [x] **Phase 50: Provider Parity & Configuration-Driven Model Swap** - Make any Paladin agent movable between the nine LLM providers by changing configuration only, or fail the build early naming the provider, model and parameter: fix the OpenAI adapter's empty user message, model parameter support per provider *and* per model (ADR amending 0004), config-driven provider construction through a public library API with all nine adapter/config pairs exported from the facade, a dated per-model profile registry feeding Commissary, response/error parity (empty output, typed refusal, usage cap, cache-write, reasoning text, stop sequences), per-step models in fallback chains, and all nine adapters in the conformance suite, live smoke and an mdBook "Switching providers" page — **executes after Phase 49, before Phases 46 and 47** (added 2026-10-14 from the second web3sec handoff) (completed 2026-10-19)
- [x] **Phase 50.1: Consumer-Surface Readiness for v0.11.0 (INSERTED)** - Close the six consumer-surface gaps web3sec's pre-measured bump found before the tag: the Aegis family (`Aegis`, `ErrorHandlerSpec`, `TimeoutPolicy`, `StateDelta`, `NodeError`, `NodeErrorSource`, `TimeoutKind`, `NodeId`, `Transience`) nameable through the `paladin` facade so MIGRATION §9.8 DR-1/2/8/9 compile without `paladin-ai-core`; one `new(..)` shape across the nine `*Config` types plus `with_timeout_seconds` (and `with_base_url`) on the shared knob builders as a registered clean break; the consumer-surface muster extended to everything the consumer actually uses with a red proof; four §9.8 gaps closed (pre-`v0.10.0` P31/P32/P33 pointers, the loss of in-adapter 429 retry and the Cadence recipe, decorator `model_profile` forwarding, the adapter-wide context window following the seeded default model); a facade-reachable pacing-only composition path; and seeds for `claude-opus-4-8` / `deepseek-v4-pro` under the seed rule — **executes after Phase 50, before Phase 51** (inserted 2026-10-19 from the fourth web3sec handoff; owner decision 2026-10-18) (completed 2026-10-20)
- [x] **Phase 51: Adjustable Reasoning Effort** - One provider-neutral `ReasoningEffort` setting per request, per agent and per provider that every adapter translates exactly to its vendor's native wire form (compile-enforced per compat preset, model-aware in the native adapters, levels read from Phase 50's model-profile registry) or refuses loudly before the HTTP call with a new additive `LlmError::UnsupportedParameter`; a silent-no-op regression test for Kimi, requested and effective effort on the response and trace stream, `Budget(n)` validated against vendor bounds and `max_tokens`, live per-vendor proof that the effort field is parsed, an ADR amending 0004 and an mdBook "Reasoning effort" page — **executes after Phase 50.1, before Phases 46 and 47** (added 2026-10-14 from the third web3sec handoff; resequenced behind the inserted Phase 50.1 on 2026-10-19 so it plans against the harmonised `*Config::new` shape; the handoff read `v0.10.1`, the feature ships in v0.11.0) (completed 2026-10-23)

Full detail: [`milestones/v0.11.0-ROADMAP.md`](milestones/v0.11.0-ROADMAP.md) ·
Audit: [`milestones/v0.11.0-MILESTONE-AUDIT.md`](milestones/v0.11.0-MILESTONE-AUDIT.md) ·
Requirements: [`milestones/v0.11.0-REQUIREMENTS.md`](milestones/v0.11.0-REQUIREMENTS.md) ·
Phase artifacts: `milestones/v0.11.0-phases/`

</details>

**🚧 v0.11.1 Dogfood Cleanup (Phases 52-58) — in progress**

*Goal: close the security, correctness, release-process and first-run gaps v0.11.0 left behind, plus whatever dogfooding v0.11.0 surfaces, so the `v0.11.1` tag ships with `WINDOWS.md` `open_count: 0` and a milestone audit that reads `passed` rather than `tech_debt`. Cleanup only: no new features, and no new ports, adapters or HTTP routes except where a security fix needs one. If Phase 53's decision point forces a persisted or public-surface break, the milestone version becomes `v0.12.0` and every `0.11.1` below reads `0.12.0`.*

- [ ] **Phase 52: CI & Release Pipeline Repair** - Re-pin the semver gate to the published `0.11.0` baseline so it checks something, drop the crate-wide allows, run the migration-allowlist, end-to-end and publish-dry-run gates before merge, fix the release-body argv cap, and clear the CodeQL, issue #41 and publish-order carry-overs
- [ ] **Phase 53: Tenant Isolation Closure** - One visibility gate for every `/v1/threads/*` route, run submission and fork refusing a foreign tenant's thread, tenant-checked job reads and cancel, redacted run-row errors, and a boot-time refusal when a configured Treasurer is not attached to the worker pool (the only S1 cluster; records the patch-versus-`v0.12.0` decision)
- [ ] **Phase 54: First-Run Usability** - Refuse unexpanded `${VAR}` HTTP auth keys at boot and make the examples honest, bring every install snippet, version line and `k8s/` label to the current release behind a `check-version-strings` gate, and fix the changelog, ADR, count and API-surface-snapshot nits a new user trips on
- [ ] **Phase 55: Correctness Fixes, Test Hygiene & Lint Bar** - No `todo!()` reachable in a published crate (held by a grep guard), the unfinished content/base-layer items each given a keep/fence/remove decision, the Failed-vs-Halted, `Cancelled`, `claude-opus-4-8` seed and unpriced-model-warning fixes, hermetic and non-flaky tests, and a warning-free lint and rustdoc bar with the eight crate-level rustdoc allows removed
- [ ] **Phase 56: Dogfood Findings** - Triage the maintainer's runs, the web3sec return and two deliberate walk-throughs into the findings log, disposition every `DF-nn`, and fix every promoted S1/S2 finding before the tag (detailed scope is unknown today and is filled from `.project/v0.11.1/02-dogfood-findings-log.md`)
- [ ] **Phase 57: Register Hygiene Close-Out** - `WINDOWS.md` at `open_count: 0`, zero undispositioned review warnings across Phases 38-51, SUMMARY frontmatter and debug sessions reconciled, the Phase 40 judgment-tier read recorded, and the three `zip` RustSec exceptions re-checked, so the audit can read `passed`
- [ ] **Phase 58: Crate Release** - Lockstep version bump held by the gate, `MIGRATION.md` consumer statement, a pre-tag milestone audit that reads `passed`, and the `v0.11.1` tag through `release.yml` with no manual step and all twelve crates on crates.io

**v0.11.1 exit criteria (the milestone's definition of done — carried by Phase 58 and re-read at the tag):**

- `WINDOWS.md` reads `open_count: 0` — every row fixed, waived with a reason, or re-homed with an owner and a date (the v0.10.0 Phase 36.1 bar that v0.11.0 did not meet).
- The milestone audit reads `passed`, not `tech_debt`.
- Zero undispositioned review warnings across Phases 38-51 (today: Phase 40's WR-01…04 and IN-04).
- `v0.11.1` ships end to end through `release.yml` with no manual step (no changelog condensation, no hand-run publish), and the semver job reports a non-zero check count.
- `README.md`, the `docs/src` install snippets, the `k8s/` labels and `openapi.json` all read `0.11.1`, enforced by a gate rather than a sweep.
- Every finding in `.project/v0.11.1/02-dogfood-findings-log.md` has a disposition (in, out with a reason, or duplicate); none is `open` at the tag.

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
[`v0.10.0-MILESTONE-AUDIT.md`](milestones/v0.10.0-MILESTONE-AUDIT.md). Phases 38-51 — the
v0.11.0 "Treasurer Spend Governance" milestone, including the four consumer-driven phases added
mid-milestone and the inserted Phase 50.1 — are archived in
[`milestones/v0.11.0-ROADMAP.md`](milestones/v0.11.0-ROADMAP.md), together with
[`v0.11.0-REQUIREMENTS.md`](milestones/v0.11.0-REQUIREMENTS.md) and
[`v0.11.0-MILESTONE-AUDIT.md`](milestones/v0.11.0-MILESTONE-AUDIT.md). Only phases in the current
and future milestones are detailed below, which is what keeps this file a constant size per
milestone. Phases 52-58 — the v0.11.1 "Dogfood Cleanup" milestone, in progress — are detailed
below.*

### Phase 52: CI & Release Pipeline Repair

**Goal**: Every later phase in this milestone, and the `v0.11.1` release itself, runs under CI and release gates that actually check something: the semver job compares against the published `0.11.0` baseline instead of reading `0 checks`, the migration-allowlist, end-to-end and publish-dry-run gates fire before the irreversible merge, and the release-body builder can no longer fail on a long changelog.
**Depends on**: Nothing (first phase of the milestone; v0.11.0 is closed and archived)
**Requirements**: CIREL-01, CIREL-02, CIREL-03, CIREL-04, CIREL-05, CIREL-06, CIREL-07, CIREL-09
**Success Criteria** (what must be TRUE):

  1. The CI semver job is named for the `0.11.0` baseline, covers all twelve publishable crates (`paladin-eval` included), and its log reports a non-zero check count instead of `0 checks: 0 pass, 254 skip`; a throwaway public-API break on a scratch branch turns it red, which proves it evaluates lints rather than skipping them.
  2. The crate-wide semver allows in `crates/paladin-core/Cargo.toml` and `crates/paladin-battalion/Cargo.toml` and their mirrored `.cargo/semver-checks-allowlist.toml` entries are gone, and the semver job stays green.
  3. On a pull request, before merge, `check-migration-allowlist`, `End-to-End Tests` and `Publish Dry Run` all run and report (on every PR or under a `release` label), so the first run on a tree is no longer after the merge.
  4. `scripts/create-or-reuse-release.sh` builds a release body larger than 131,072 bytes without passing it through argv, proven by a regression test whose body exceeds that size, so no manual changelog condensation is ever needed again.
  5. Each of the 24 advisory CodeQL alerts on the release PR heads carries a dismissal reason in `CODEQL-DISMISSALS.md` (or is fixed) with the register check green and the advisory-only posture unchanged; GitHub issue `DF3NDR/paladin-dev-env#41` is closed with a reference to the `v0.11.0` GitHub Release (or restated with what is still missing); and the publish-order gate reading (`ci.yml` `Check publish order` plus the dependency-ordered publish job) is recorded as accepted in the release documentation.

**Plans**: 8 plans

Plans:
**Wave 1**

- [ ] 52-01-PLAN.md — Semver gate re-pinned to 0.11.0 at patch through `scripts/check-semver.sh` (twelve crates, zero-check tripwire); allowlist gate moved into the required job (CIREL-01, CIREL-03)
- [ ] 52-02-PLAN.md — Release body via `jq --rawfile`, shared character-counting size helper, create-time truncation; >131,072-byte regression proven red first (CIREL-05)
- [ ] 52-03-PLAN.md — CodeQL alert triage into CODEQL-DISMISSALS.md and issue #41 close, with maintainer intake checkpoint (CIREL-06, CIREL-07)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 52-04-PLAN.md — Finalize-release-body size guard, idempotent re-fit of the curated part (CIREL-05)
- [ ] 52-05-PLAN.md — All six crate-wide semver allow tables removed, measured; MIGRATION.md §11 opened with an empty §11.2 register the gate reads (CIREL-01, CIREL-02, CIREL-03)
- [ ] 52-06-PLAN.md — Publish Dry Run on every PR, End-to-End Tests under the `release` label; trigger register, committed ruleset record and release docs (publish-order reading) updated (CIREL-04, CIREL-09)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 52-07-PLAN.md — Semver red proof: local leaf-crate break, then scratch-branch draft PR after the maintainer's go-ahead (CIREL-01)

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 52-08-PLAN.md — Phase gate run, folded coverage-reproduction record, maintainer hand-off checklist (ruleset PUT, label run, CodeQL dismissals) and outcomes (CIREL-01, CIREL-04, CIREL-06, CIREL-07)

**Scope notes**:

- The re-pin must make the tool evaluate lints at the patch release type (an explicit `--release-type` or the manifest bump): the `0.9.0` baseline went blind because the manifest version delta was major-equivalent, so a re-pin that again reads zero checks is not done. From here on a public-API change in a later phase fails red at patch level; Phase 53's and Phase 55's signature or enum changes are judged by this gate and feed the `v0.11.1`-versus-`v0.12.0` decision.
- The one known finding against `0.10.1` (`enum_struct_variant_field_added` on `RunOutcome::Halted.cause`) is expected to disappear at the `0.11.0` baseline; any other finding the real gate now surfaces is triaged in this phase (fix, or a registered allowlist entry with its `MIGRATION.md` row), not carried.
- CIREL-08 (the eight crate-level `#![allow(rustdoc::…)]`) is deliberately mapped to Phase 55, not here: removing them can surface an unbounded number of rustdoc warnings across five crates and the facade, and that must not delay arming the gates every other phase relies on.

### Phase 53: Tenant Isolation Closure

**Goal**: A principal of one tenant can no longer enumerate, read, run against or mutate another tenant's threads, runs or jobs, and a configured Treasurer can no longer be silently dropped by worker-pool wiring. This is the milestone's only S1 cluster, and it is bounded.
**Depends on**: Phase 52 (the armed semver gate judges the fixes' public-surface effect)
**Requirements**: TENANT-03, TENANT-04, TENANT-05, TENANT-06, TENANT-07, TENANT-08, TENANT-09, TENANT-10
**Success Criteria** (what must be TRUE):

  1. A bearer of tenant B calling any `/v1/threads/*` route (list, get, state, history, resume, fork, delete) on tenant A's thread gets the same 404 a missing thread yields, `GET /v1/threads` omits A's threads, and a route matrix built from the router's own OpenAPI document fails on any future `/v1/threads/{id}*` route that skips the one visibility gate.
  2. Tenant B cannot run against or fork tenant A's thread (`POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork` answer 404), cannot read A's job through `GET /agents/{id}/jobs/{job_id}`, and a library caller of `RunSubmissionService::cancel` presenting a foreign tenant is refused at the service layer with no HTTP controller in the path.
  3. A corrupt or half-attributed run row, and any `Serialization` error, reaches an API client as a fixed redacted error with no backend text, while a foreign tenant still receives the missing-run 404 (hidden equals missing).
  4. With a Treasurer configured, `paladin-server` refuses to boot unless the worker pool was built through `with_engine_factory` (a test proves the refusal); reserved literals are rejected as tenant and API-key names at config load; and boot logs exactly one `warn!` listing schedules with `NULL created_by`, with no backfill route.
  5. The patch-versus-minor decision is recorded in this phase before Phase 58 opens: either the fixes stay additive (semver job green at patch level, no `MIGRATION.md` break; a tenant column on threads is a migration, not a break) or the one deliberate break is named, registered in `MIGRATION.md` and the allowlist, and the milestone version becomes `v0.12.0`.

**Plans**: TBD
**Scope notes**:

- **Re-verify against the tree first.** Phase 40's `40-REVIEW-FIX.md` records WR-01…WR-04 (TENANT-04, -05, -06) as fixed: `ensure_thread_visible` is in `src/application/services/run/submission.rs`, `RunSubmissionService::cancel` enforces the tenant in the service, and `run_controller.rs` returns a fixed run-store error message. The carried-gaps inventory (built from `40-REVIEW.md` alone) lists them open. For those three requirements the plan starts by proving the criterion against the tree and adds the regression test where one is missing; only what fails is rebuilt. TENANT-03, -07, -08, -09 and -10 are expected to be real work.
- **Decision point (v0.11.1 or v0.12.0).** A tenant column on threads, or a signature change on a published type, may force a persisted or public-surface break. Decide when the break is known, not at the tag: if it is forced by this security fix it is the one deliberate break (`MIGRATION.md` §11, mirrored allowlist entry) and the version is `v0.12.0`; a break not forced by security is reshaped or dropped instead. Shared-gate design (derive the tenant from a thread's runs versus record it on the thread) is a discuss-phase question.
- No new HTTP routes: the fixes route existing handlers through the gate.

### Phase 54: First-Run Usability

**Goal**: A new user who copies from the README, the docs, `config.example.yml` or `k8s/` gets a working first run: no example loads a placeholder as a credential, every install snippet and version line reads the current release, and a gate, not a one-off sweep, keeps it that way.
**Depends on**: Phase 53 (FIRST-01 and TENANT-09 both extend `AuthConfig` load-time validation, so sequencing keeps those changes in one reviewed order), and through it Phase 52's armed gates
**Requirements**: FIRST-01, FIRST-02, FIRST-03, FIRST-04, FIRST-05, FIRST-06, FIRST-07, FIRST-08, FIRST-09
**Success Criteria** (what must be TRUE):

  1. Starting `paladin-server` with an `http.auth.api_keys[].key` that is an unexpanded `${…}` placeholder, as `config.example.yml` and `k8s/server/configmap.yaml` carry today, exits at config load with an error naming the key; both examples no longer carry a loadable placeholder key, and the configmap header no longer claims load-time expansion.
  2. Every install snippet and version line a user copies from reads the workspace's current version: `README.md` (the `"0.5"` pin and `Current version`), `docs/src/appendix/sentinel.md`, `crate-map.md`, `feature-flags.md`, and the `version:` labels in `k8s/server/deployment.yaml` and `worker-deployment.yaml`; historical rows inside the migration guide are exempt.
  3. `make check-gates` and CI fail when any user-facing version string (README, `docs/src`, `k8s/`, `openapi.json`) lags the workspace version, shown by a deliberately stale string turning the gate red.
  4. The `[0.11.1]` entries of the `paladin-memory` and `paladin-content` changelogs correct the `[0.11.0]` "No functional change" wording with the one-way `paladin.node` → `paladin.data` persisted-key change; ADR-0056 and the treasurer tests' doc comment cite ADR-0057 (check-only) rather than a reservation; and the stale "ten crates" counts, the `cargo-release` version line and the dangling `[Unreleased]` link at `CHANGELOG.md:3396` are corrected.
  5. `.project/current-exports.txt` is regenerated by `make api-surface` on the pinned nightly and carries the `UnsupportedParameter` / `check_reasoning` rows; the node-cache guide states that the server attaches no node cache (the PACE-04 stampede lock is a library builder only) and documents the cache-key version and the `key_prefix` rotation for a provider-level default-effort change.

**Plans**: TBD
**Scope notes**:

- Version strings are held to the workspace manifest version: they read `0.11.0` while the manifest does, and the same gate is what forces the lockstep bump to `0.11.1` in Phase 58, where a stale string is a red gate rather than a missed sweep.
- The API-surface snapshot is regenerated here and again by whichever later phase moves the public surface (the CLAUDE.md pre-commit protocol); Phase 58 re-checks it on the release commit.
- `${VAR}` expansion is out by decision (2026-10-27): no new secret-handling path; the LLM `api_key: "${OPENAI_API_KEY}"` indirection is a separate, working mechanism and is untouched.

### Phase 55: Correctness Fixes, Test Hygiene & Lint Bar

**Goal**: Nothing in a published crate panics from an unfinished stub or misreports a status, the unfinished content/base-layer code is fenced with a recorded decision, and the test suite and lint bar are hermetic, non-flaky and warning-free, so the later phases and the release run on a clean baseline.
**Depends on**: Phase 52 (the armed semver gate judges the public-signature changes here) and Phase 53 (FIXES-06's boot warning sits beside TENANT-08's boot assertion)
**Requirements**: FIXES-01, FIXES-02, FIXES-03, FIXES-04, FIXES-05, FIXES-06, TESTS-01, TESTS-02, TESTS-03, TESTS-04, TESTS-05, TESTS-06, CIREL-08
**Success Criteria** (what must be TRUE):

  1. No `todo!()` or `unimplemented!()` is reachable outside `#[cfg(test)]` in any published crate (doc-comment examples excepted), held by a grep guard in the gates that fails on a planted one; the previously panicking stubs return a typed error or are gone, and every kept-but-unfinished content / general-data base-layer or content-processing item (`Field`, `Collection`, `element`, the two versioning services, `planning_service.rs:417`, `file_content_fetcher.rs`) has a recorded keep / fence / remove decision and refuses cleanly with rustdoc saying it is unfinished.
  2. A `DispatchBudget::Unpriced` outcome on an agent-kind run records `Halted`, not `Failed` (or the Treasurer guide documents why the mapping is not a local change); an eval scenario can assert `run_status: cancelled`; the `claude-opus-4-8` seeded `reasoning` flag matches the Phase 50.1 probe record; and server boot warns once per model reachable by a metered principal that has no `treasurer.pricing` row, without refusing workflow assistants.
  3. `cargo test` passes in a sandbox with no network, including the two `run_api_wiring::tests::build_run_api_persists_*` tests against a scripted port; `cancel_tests::local_cancel_signals_token` waits on a dispatch-registration signal rather than a fixed sleep and survives repeated runs under parallel load; and WINDOWS row 73 is dispositioned.
  4. Single-feature `cargo clippy --tests` on `paladin-llm` is warning-free, the Phase 51 lint leftovers are cleared, and `cargo doc` is warning-free under both CI bars with the eight crate-level `#![allow(rustdoc::…)]` lines removed and `doc-coverage-report.md` regenerated.
  5. The self-hosted provider pair (`ollama`, `openai-compatible`) is defined once, or 51 IN-01 is closed as not-reproducing with the evidence recorded, and the `commander.rs` "TODO: Enhance MockPaladinPort" stubs are deleted or point at `tests/integration/commander_error_paths_test.rs`.

**Plans**: TBD
**Scope notes**:

- **FIXES-01 reachability (measured at roadmap time).** The file the requirement names, `crates/paladin-content/src/services/content_filtering_service.rs`, is commented out of `services/mod.rs` (`// pub mod content_filtering_service;`) and is not valid Rust (`content..contains(keyword)`), so that `ContentFilter` panic is not reachable today. The compiled, reachable stubs are the twenty `todo!()` bodies in `impl ContentListRepository` for `SqliteStore` (`crates/paladin-storage/src/sqlite_content_repository.rs`, feature `sqlite`) and `MySqlContentRepository` (`mysql_content_repository.rs`, feature `mysql`); all return `Result<_, RepositoryError>`, so a typed `Err` is a body-only change. The grep guard is the real bar; plan-phase re-measures (feature-gated sites included) before sizing.
- **Semver decision for FIXES-04.** `RunStatusValue` in `paladin-eval` is not `#[non_exhaustive]`, so adding `Cancelled` is a variant addition the Phase 52 gate will flag at patch level, and the scope guard forbids a `MIGRATION.md` break that is not forced by security. Settle at discuss-phase: shape it without a public-enum change, carry it as a registered break (which makes the milestone `v0.12.0`), or move FIXES-04 to the next milestone with a recorded reason. The same question applies to any other public signature the FIXES items touch.
- **Ordering.** TESTS-01 (a hermetic scripted port for the agent-kind run path) lands before FIXES-03, whose regression test rides on it.
- **CIREL-08 placement.** Removing the eight rustdoc allows shares its bar and files with TESTS-03 and TESTS-04 (the `reasoning.rs` redundant links and the `cache_key.rs` link are the same warning class); the all-features CI test job stays out by decision.

### Phase 56: Dogfood Findings

**Goal**: The defects v0.11.0 actually shows when used for real, in the maintainer's own runs, on the published `0.11.0` crates and in the web3sec consumer's return, are found, logged, dispositioned and, where S1 or S2, fixed before the tag.
**Depends on**: Phase 55 (so findings can be deduplicated against fixes already made); the maintainer's runs and the web3sec return may arrive earlier and are appended to the log as they come
**Requirements**: DOGF-01, DOGF-02, DOGF-03, DOGF-04, DOGF-05, DOGF-06
**Success Criteria** (what must be TRUE):

  1. Every `DF-nn` entry in `.project/v0.11.1/02-dogfood-findings-log.md`, including the pre-seeded DF-01…DF-04, carries a disposition (`in v0.11.1`, `out` with a reason, or `duplicate of`); none reads `open`.
  2. The web3sec consumer's return against `v0.11.0` (`47-HANDOFF-web3sec.md` §6: commit SHA, D-2 choice, gate totals, divergences) is triaged into the log with one entry per divergence; if the return has not arrived, DOGF-02 is re-homed with a named owner and a date by the maintainer.
  3. Each section of `config.example.yml` round-trips through the config loader with every optional key present, as a passing test, and every key the loader rejects or silently ignores is logged as a finding.
  4. The first-run path is walked on the published `0.11.0` crates (README Quick Example → `cargo add` → `basic_paladin`, then `paladin-cli onboarding` and `setup-check` against one real provider key) and every stumble is logged as a finding; the maintainer's local `make coverage` walk of `docs/src/contributing/testing-guide.md` on a Docker-capable machine closes todo `2026-08-13-verify-local-coverage-reproduction` with the measured figure.
  5. Every finding promoted to `in v0.11.1` at severity S1 or S2 is fixed and verified before this phase closes, in plans here or in decimal phases (`56.1`+) inserted once the findings are known; none remains open.

**Plans**: TBD
**Scope notes**:

- **Detailed scope is unknown today.** This phase is a fixed frame around unknown content: its plans are written from `.project/v0.11.1/02-dogfood-findings-log.md` once the maintainer's runs and the web3sec return have arrived. Today the log holds only DF-01…DF-04, all pre-seeded from reading the tree and all already owned by earlier phases (DF-01 → FIRST-01, DF-02 → FIRST-02/03, DF-03 → FIXES-01, DF-04 → TENANT-03…05 and TENANT-07), so they are dispositioned here `in v0.11.1` with a pointer to that owner.
- Decimal fix phases inserted under this phase (`/gsd-phase --insert`) inherit DOGF-06 and do not change the requirement mapping.
- DOGF-04 and DOGF-05 need the maintainer's machine, a real provider key and (DOGF-05) Docker; they are human-gated checkpoints, not agent-run steps. If the web3sec return is still outstanding when everything else is done, the maintainer either holds the tag or re-homes DOGF-02, since SHIP-10 admits only fixed, waived-with-reason or re-homed-with-owner-and-date.
- A finding that collides with an accepted contract (inventory §H: ADR-0057 check-only boundary, streamed-call overshoot, agent-loop no-Waypoint, the OBS-05 waiver) is logged with the ADR reference and dispositioned `out`, not reopened. Provider measurement work (Ollama / OpenAI-compatible probes, quota-body capture) enters only if a run hands over a captured body.

### Phase 57: Register Hygiene Close-Out

**Goal**: Every register the milestone audit reads agrees with the tree: no open `WINDOWS.md` row, no undispositioned review warning, no stale frontmatter or diagnosed-but-fixed debug session, and no unchecked advisory exception, so the audit can read `passed` rather than `tech_debt`.
**Depends on**: Phase 56 (dogfood findings and fixes can mint or close `WINDOWS.md` rows) and Phase 53 (the Phase 40 warnings close on its evidence)
**Requirements**: REG-01, REG-02, REG-03, REG-04, REG-05, REG-06
**Success Criteria** (what must be TRUE):

  1. `windows status` reports `open_count: 0`: rows 56, 68, 69 and 79 are dispositioned with their superseding evidence, rows 58, 62, 63, 67 and 73 are closed on the evidence of Phases 53-55 or waived with a reason, every other open row is waived with a reason or re-homed with an owner and a date, and any row minted by Phases 52-56 is included.
  2. A sweep of every `NN-REVIEW.md` for Phases 38-51 shows a disposition beside each warning; Phase 40's WR-01…WR-04 and IN-04 are closed (by `40-REVIEW-FIX.md` and the Phase 53 regression tests, TENANT-07 for IN-04, TENANT-09 for IN-01) or waived with a reason, leaving zero undispositioned review warnings.
  3. The Phase 45 and Phase 51 SUMMARY `requirements-completed` frontmatter carries STORE-02 and REASON-02/03/05/06, and the three `diagnosed` sessions in `.planning/debug/` (kimi fallback profile, openai `max_completion_tokens`, vendor token usage fidelity) are archived as resolved.
  4. A bounded human read of the Phase 40 judgment-tier prohibitions that carry LLM-judge verdicts is recorded in that phase's register, and the Phase 42 subset is recorded as accepted as-is.
  5. The three `zip 3.0.0` RustSec exceptions in `SECURITY-EXCEPTIONS.md` are re-checked against the current `utoipa-swagger-ui` release and either lifted or re-dated with the check recorded, with `make security` green.

**Plans**: TBD
**Scope notes**:

- Mostly planning-register work with little code. Inventory F1 (nine phases NOT-VALIDATED) was found already closed at the milestone open (all fifteen Phase 38-51 `VALIDATION.md` files read `status: validated`) and is not a requirement.
- The open rows at the v0.11.0 close are 56, 58, 62-69, 73, 77 and 79. Rows 64-66 are accepted contracts (ADR-0057, inventory §H) and row 77 is the owner's Grok vendor-cost deferral; REG-02's `open_count: 0` clause requires each to be waived with a written reason or re-homed with an owner and a date even though no other requirement names them.

### Phase 58: Crate Release

**Goal**: `v0.11.1` is cut through the repaired pipeline: version strings bumped in lockstep and held by the gate, the consumer upgrade story stated honestly, the milestone audit read as `passed` before the tag, and all twelve crates on crates.io with no manual step.
**Depends on**: Phase 57 (and Phase 53's recorded version decision)
**Requirements**: SHIP-08, SHIP-09, SHIP-10
**Success Criteria** (what must be TRUE):

  1. The workspace, all twelve publishable manifests and the changelogs read `0.11.1`, and `make check-gates` (including `check-version-strings`), `make clean-code`, `make security` and `make api-surface` are green on the release commit, with every user-facing version string (README, `docs/src`, `k8s/`, `openapi.json`) reading `0.11.1` because the gate demands it, not because a sweep remembered.
  2. `MIGRATION.md` states that a `0.11.0` consumer takes `0.11.1` without adaptation, or records the one deliberate break, in which case the version is `v0.12.0` and not a quietly breaking patch.
  3. The milestone audit over Phases 52-57 reads `passed`, not `tech_debt`: every carried item is fixed, waived with a reason, or re-homed with an owner and a date; `windows status` still reads `open_count: 0`; and no review warning in Phases 38-51 is undispositioned.
  4. `v0.11.1` is tagged on `main` and ships through `release.yml` end to end with no manual step (no changelog condensation, no hand-run publish), and the semver job read a non-zero check count on the release pull request.
  5. All twelve publishable crates are on crates.io at `0.11.1`, unyanked and published through Trusted Publishing as read from the registry after the run, and the GitHub Release exists with its assets.

**Plans**: TBD
**Scope notes**:

- Suggested ordering: lockstep bump (the Phase 54 gate goes red until every string moves) → release pull request with the Phase 52 gates on → pre-tag audit of Phases 52-57 reads `passed` → tag via `release.yml` → registry read. Phase 58's own proof is the post-tag registry read, so the audit is not circular. The merge, the tag and the publish remain maintainer-gated checkpoints, as in Phase 47.
- If Phase 53 recorded a forced break, every version in this phase reads `0.12.0` and SHIP-09 records the break instead of the no-adaptation statement.

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
| 38-51 | v0.11.0 | 174/174 | ✅ Shipped | 2026-10-27 |
| 52. CI & Release Pipeline Repair | v0.11.1 | 0/TBD | Not started | - |
| 53. Tenant Isolation Closure | v0.11.1 | 0/TBD | Not started | - |
| 54. First-Run Usability | v0.11.1 | 0/TBD | Not started | - |
| 55. Correctness Fixes, Test Hygiene & Lint Bar | v0.11.1 | 0/TBD | Not started | - |
| 56. Dogfood Findings | v0.11.1 | 0/TBD | Not started | - |
| 57. Register Hygiene Close-Out | v0.11.1 | 0/TBD | Not started | - |
| 58. Crate Release | v0.11.1 | 0/TBD | Not started | - |

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

**v0.11.0 shipped 2026-10-27:** 15 phases (38-51, with 50.1 inserted and 48-51 added mid-milestone),
174 plans, 281 tasks, 63/66 requirements (ADOPT-01…03 superseded at source), 34 days
(2026-09-24 → 2026-10-27; 1,126 commits on the release PR #59 per GitHub, `df24689c` → `70657919` →
`a49200da`). Audit status `tech_debt` — no blockers; see
[`milestones/v0.11.0-MILESTONE-AUDIT.md`](milestones/v0.11.0-MILESTONE-AUDIT.md). Released on one
tag: `v0.11.0` on merge commit `70657919` (2026-10-26, release run `40396108246`) published all
twelve crates at `0.11.0`; `Coverage` 92.74 % on that commit. Closeout type `override_closeout`
(0 unverified phases; 9 open artifacts acknowledged — three root-cause-only debug sessions whose fixes
landed in Phase 50, two tooling false positives, four pending todos). See MILESTONES.md.

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
  ingested requirement demands it. *2026-10-24, Phase 46: waived -- WINDOWS row 80.*

- **Clone/lock-contention work** — the 383 `.clone()` calls and nine orchestrator locks flagged in
  `codebase/CONCERNS.md`. Blocked on Phase 3 producing benchmark evidence first. *2026-10-24, Phase 46: waived -- WINDOWS row 81.*

- **The `paladin-core` / `paladin-ports` dependency allowlists** — declared 6 and 7, shipping 14 and
  10. The architectural invariant holds; this is document-versus-code drift needing ARCH-03(b) to
  choose a direction. *2026-10-24, Phase 46: fixed -- ADR-0015 re-measured at 14 / 12 and pinned by `scripts/check-dependency-allowlist.sh`; WINDOWS row 82.*

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

*Closed: 2026-10-27 — **v0.11.0 "Treasurer Spend Governance" shipped and archived.** Phases 38-51
(15 phases: the ten roadmapped 2026-09-24 plus Phase 48 added 2026-10-12, Phases 49-51 added
2026-10-14 from the three web3sec handoffs, and Phase 50.1 inserted 2026-10-19 from the fourth) are
collapsed above and archived in full under `milestones/v0.11.0-*`. Every `### Phase N:` header and
every per-plan checklist is preserved verbatim in the archive. Phases 1-37.1 unchanged and
unrenumbered. The next milestone has not been defined; its phases start at Phase 52.*
