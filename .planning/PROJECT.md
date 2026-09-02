# Paladin

## What This Is

Paladin (root crate `paladin-ai`) is a Rust workspace that provides an enterprise AI
orchestration framework: autonomous agents (**Paladins**) coordinated through eight multi-agent
patterns (**Formation** sequential, **Phalanx** concurrent, **Campaign** DAG, **Chain of
Command** hierarchical, **Conclave** expert synthesis, **Council** group discussion, **Grove**
tree-based routing, **Maneuver** Flow DSL), assembled behind hexagonal ports so that pluggable LLM
providers (OpenAI, Anthropic, DeepSeek, Mock), MCP tool servers, short-term conversation memory
(**Garrison**), long-term semantic memory with RAG (**Sanctum**), multi-modal vision
(**Sentinel**), state persistence (**Citadel**), output formatters (**Herald**), a CLI
(**Armory**) and an HTTP API are all swappable adapters.

Its audience is Rust developers and teams embedding agent orchestration inside their own
services — not end users of a hosted product.

The product **already works**. It is a brownfield project at v0.7.0 (**amended at the v0.9.0
close, 2026-09-01**: the manifests now read `0.8.0`, all eleven publishable crates are live on
crates.io at `0.8.1-rc.x` prereleases via OIDC Trusted Publishing, and the release pipeline has
had its first fully-green end-to-end run — the next real release sets the next version) with a
Cargo workspace of
**ten library crates** (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`,
`paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`,
`paladin-web`) plus a `doc-examples` crate and the root facade package `paladin-ai`, 22 runnable
examples, an HTTP API with OpenAPI and SSE streaming, a `paladin-server` binary, a multi-arch
Docker image and reference Kubernetes manifests. (**Amended by Phase 4, dated 2026-08-03, citing
`04-release-measurement.md`**: "22 runnable examples" traces to a Milestone 1 Epic 10 validation
report ("22/22 examples compiling") and is stale — the shipped tree carries **47** `.rs` files
under `examples/`, 4 of them declared `[[example]]` targets gating on non-default features
(`vision`, `content-processing`, `web-server`); no crate under `crates/` ships its own
`examples/` directory. The shipped tree outranks an ingested count under this project's precedence
order. The gate REL-05 now expresses is "every example target builds", not a count.)

## What This Planning Corpus Is

**A historical record of twelve shipped milestones, plus a verified-defect and deferred-work
forward scope. Not a greenfield plan, and not a backlog.**

The ingest is complete: five runs covered all **263** documents in `.project/` — **199** classified
(188 prose documents + 11 task lists) and **64** `tasks-*.md` measured deterministically by
`intel/task-completion-state.md`. That produced **554 requirements**, **103 context topics**,
**0 blockers**, **69 preserved competing variants** and **0 locked decisions**.

Nine of the twelve milestones are at or above 98% complete by checkbox, and every one of them is
corroborated or exceeded by the shipped tree. **The forward scope in `.planning/` exists because
five runs of direct code verification found a small number of things that are genuinely broken,
genuinely unbuilt, or genuinely undecided** — not because the milestones are unfinished:

- **Verified defects** — a permanently red `api-surface` CI job, a duplicate `cargo audit` job that
  falsifies a completed milestone's own success metric, zero `#[deprecated]` annotations against a
  requirement that demands them, disabled port doctests, three CLI dependencies leaking into
  library builds, three `TokenUsage` structs where the record names one, a capability flag that
  over-reports.
- **Genuinely unbuilt** — `Deferred-QA-CICD-Completion` Epics 25-29, the only ingested epic-set in
  the corpus verified open *item by item* rather than inferred.
- **Genuinely undecided** — the token mechanism, the multi-replica token store, the licence
  posture, the leaf-crate dependency rule, the PDF capability, the `cargo doc` bar, the
  `AgentProvisioner` placement, and the fate of a 311-line architecture document.

**The arc has a shape worth naming.** M1-M3 built capability. M4-M8 dismantled and rebuilt the
structure that capability lived in, at considerable cost and with almost no feature work. M9
finished the half of the platform M4-M8 had left alone. M10-M11 made it releasable and documented.
M12 exposed it over HTTP — and it exists because M11's documentation epic wrote down a capability
gap instead of papering over it.

*(Corrected by ingest run 3: this file previously said "9-crate workspace", and the Milestone 5/6
source documents assume six. The tree was read directly — `crates/` holds eleven directories, ten
of them library crates. **Closed by ingest run 4:** all ten now have an ingested requirement.
`paladin-storage`, `paladin-notifications`, `paladin-content` and `paladin-web` come from Milestone
7 Epic 1's extraction PRD and its cost-benefit gate; `paladin-herald` was created by the 2026-06-04
facade-cleanup reconciliation rather than by any PRD — inside an Epic whose non-goals named it as
out of scope — which is exactly why no ingested requirement described it and why the "9 crates"
figure was wrong. **Closed completely by ingest run 5:** the last shipped subsystem without an
ingested requirement — Milestone 12's Axum HTTP API surface, including auth, rate limiting, OpenAPI
and SSE streaming — now has 34 requirements across seven Epics. **Every shipped subsystem in this
workspace now has at least one ingested requirement behind it.**)*

## Core Value

A Rust developer can compose and run multi-agent workflows against any supported LLM provider
through stable port abstractions — without their own domain code depending on a provider,
transport, or storage implementation.

## Success Metric (derived — not user-specified)

No developer-facing success metric was supplied. This one is derived strictly from the
measured evidence in the ingest set (`Epic_10/task6.0-validation-report.md`, dated
2026-01-27; `unit-test-improvements/COVERAGE_ANALYSIS.md`; and
`Milestone_3-Completion/RELEASE_NOTES_MILESTONE_3.md`):

**On a clean clone of the release branch, `cargo fmt --check`, `cargo clippy -- -D warnings`
and `cargo test --workspace` all pass with zero failures and zero warnings — and `cargo llvm-cov`
reports a coverage figure at or above whichever gate this project records, up from the measured
60.88% unit / 67.79% integration Milestone-1 baseline and the ~78% overall figure reported at
Milestone 3.**

Why not something more ambitious: the evidence supports claims about build, lint, test and
measured-coverage state. It does not support performance, throughput, or onboarding-speed
claims. The Milestone-1 benchmark suites were disabled; benchmarks now exist per-crate but no
baseline document has been produced, so no verified performance number exists. The
"< 15 minutes to first working agent" figure in Epic 10's PRD and the "90% of new users run
their first agent within 5 minutes" figure in Epic 18's PRD were both never measured. Those
remain documented *targets*, tracked in Phase 3 and Phase 4 respectively — not the current metric.

Reported test totals are deliberately excluded from the metric: across the corpus they run
999 → 1,292 → 1,674 → 1,628 → 853, i.e. not a monotonic series, so no single figure is
trustworthy enough to anchor a gate.

## Current State

**Phase 22 complete (2026-09-02)** — battlefield-state-superstep-engine, the v0.10.0 milestone's
first phase: typed Battlefield state, the superstep WarEngine with checkpoint/resume over three
WaypointPort backends (the Postgres Tier 2 contract suite now provably executes in CI — run
33688238662), eligible-set reachability validation (BUG-02 closed), and crash-safe retention on
the prune_thread primitive. 17/17 plans, all three UAT gaps closed. Residuals routed to inserted
Phase 22.1: the frontier readiness defect, the MSRV-1.85 vs rmcp-pinned process-wrap decision,
and CR-01 (fingerprint omits defer_flags).

**Shipped: v0.9.0 "Security Tooling" (2026-09-01)** — Phases 18-21, 25 plans, 20/20 requirements
(SAST-01…04, PUB-01…05, PUBOPS-01…05, ARTIFACT-01…06). Audit status `tech_debt`, no blockers:
`.planning/milestones/v0.9.0-MILESTONE-AUDIT.md`.

**Released for real on 2026-09-01, hours after the close** (superseding the close-time "no tag
was cut" note): at the user's direction, release numbers were reconciled with milestone names.
PR #50 bumped all twelve manifests `0.8.1-rc.5` → `0.9.0` and curated the changelog; tag
`v0.9.0` was cut on merge commit `0b5d4106`; release run `33542459191` completed fully green —
**all eleven crates published to crates.io at `0.9.0`** via Trusted Publishing
(registry-verified), with a stable GitHub release carrying four binary archives, per-asset
checksums, `SHA256SUMS`, the SBOM, and a digest-pinned image reference. **First stable release
since 0.5.1 (2026-06-04)** — 0.6.0, 0.7.0 and 0.8.0 were finalized in the changelog but never
released, and the 0.8.1-rc line never graduates. From v0.9.0 forward, release versions track
milestone names.

What v0.9.0 settled, in one paragraph each:

- **The Rust-SAST question is answered by measurement, not adoption.** CodeQL provably analyses
  all 385 first-party `.rs` files — the exact coverage proof whose absence disqualified Snyk —
  and was then itself disqualified as a required-check-grade Rust SAST (version-scoped: CodeQL
  `2.26.3` / `rust-queries` `0.1.40`; SQL injection, path traversal and regex injection never
  fired across four independent measurements). `codeql.yml` is retained advisory-only behind a
  governed dismissal register, and the manual credential-handling review remains the primary
  control, stated plainly everywhere the gap was previously recorded.
- **Publishing runs on ephemeral OIDC credentials.** All eleven crates publish through
  per-run tokens minted under the protected `crates-io` environment; the long-lived "Paladin"
  token is revoked at the registry (operator-confirmed) and the `CARGO_REGISTRY_TOKEN` secret is
  deleted — proven in ratchet order by a real `0.8.1-rc.2` publish before anything was revoked.
- **A release is now gated, idempotent, and recoverable.** A pre-publish consistency gate (tag ↔
  eleven manifests ↔ eleven changelogs ↔ recorded CI conclusion) structurally blocks
  `cargo publish`; a same-tag re-run is the supported recovery and reaches the publish step;
  already-published is read from registry state; a run that moves nothing fails; and the
  stuck-halfway runbook with its yank policy was rehearsed live (v0.8.1-rc.3/rc.4).
- **A release hands a consumer something real.** Curated `CHANGELOG.md` body (missing section
  fails the run), binaries that actually build under their required features, an image pinned by
  immutable digest, aggregated `SHA256SUMS` with one-command verification, SBOM scope stated —
  proven end-to-end on `v0.8.1-rc.5`, the first fully-green release run in this project's
  history, with every declared human check closed by recorded UAT.

## Current Milestone: v0.10.0 Durable Agent Execution Runtime

**Goal:** Evolve Paladin from a pattern-oriented orchestration framework into a durable agent
execution runtime — typed shared state, automatic per-superstep checkpointing with crash-resume,
human-in-the-loop pause/resume, dynamic control flow, per-node fault tolerance, a background-run
platform API, and standardized trace observability.

**Source of truth:** the approved design corpus in `.project/v0.10.0/` — program overview (00),
seven epic PRDs (01-07, epic prefixes `ENG`, `CF`, `HITL`, `FT`, `RT`, `PLAT`, `OBS`), and a
gap-analysis traceability matrix (08). The PRDs carry FR-level behavior (~135 FRs) and remain the
behavior source of truth; `.planning/REQUIREMENTS.md` scopes them as capability clusters with
explicit FR-range traceability. Cross-cutting rules X-01…X-11 (hexagonal boundaries, TDD with the
82% coverage floor, X-03 backward compatibility, X-10 semver hygiene via `cargo semver-checks`,
X-11 MSRV 1.85 discipline) apply to every epic, and a root-level `MIGRATION.md` (overview §9) is
a program-level living deliverable created in the first epic.

**Implementation order (overview §2):** 01 → 02 → 03 ∥ 04 → 05 (parallelizable with 03/04)
→ 06 → 07, with standalone items schedulable earlier. New phases start at Phase 22.

**Target features:**
- Battlefield typed state + superstep engine: cycles with bounded iteration, deterministic merge,
  automatic Waypoint checkpointing, resume with zero re-execution (Doc 01, keystone)
- Dynamic control flow: BUG-01 fail-closed fix, Directive node-driven routing, Muster fan-out,
  subgraph composition, LLM-evaluated routing (Doc 02)
- Parley pause/resume, Chronicle history/replay/fork, graceful shutdown (Doc 03)
- Aegis per-node fault tolerance: transience taxonomy, retry/timeout/typed error handlers, model
  fallback, node caching (Doc 04)
- Agent runtime: execution middleware chain, context-window management, Vault long-term memory,
  structured output, provider-conformance close-out (Doc 05)
- Platform API: background runs on a durable queue, threads over HTTP, versioned assistants,
  cron schedules, signed webhooks with SSRF guard (Doc 06)
- Observability: TraceSink event stream, OTel export, graph/execution visualization,
  `paladin-eval` harness (Doc 07)
- Program gates: `MIGRATION.md` complete per §9, semver + MSRV CI jobs, three E2E acceptance
  scenarios (crash-resume, approval gate, dynamic map-reduce), v0.10.0 release readiness

**Known PRD-vs-tree conflict (precedence: shipped tree outranks PRD):** PRD 05 §1/§2.5 assumes
provider coverage is "OpenAI/Anthropic/DeepSeek only" and specifies an OpenAI-compatible generic
adapter (RT-FR-20), a Gemini adapter (RT-FR-21) and an Ollama recipe (RT-FR-22) as new work — all
three shipped in v0.8.0 (PROV-01…04; `crates/paladin-llm/src/{openai_compatible,gemini,ollama}`,
facade features `llm-openai-compatible`/`llm-gemini`/`llm-ollama` live at `Cargo.toml:307-309`).
RT-06 scopes these as verify-against-the-PRD's-conformance-bar and close gaps (shared conformance
suite, FT-FR-01 transience mapping, documented Ollama recipe), not as greenfield builds.

Carried-in open items (unchanged from the v0.9.0 close; tracked, not this milestone's scope
unless a phase adopts them):

- The user-owned local coverage-reproduction walkthrough (STATE.md *Deferred Items*, unchanged
  since the v0.8.0 close).
- Nyquist validation unreconciled for all archived phases 05-21 (`/gsd-validate-phase <N>`).
- The five v0.9.0 debt items inventoried in `milestones/v0.9.0-MILESTONE-AUDIT.md`: the CodeQL
  re-probe trigger condition, the untested `workflow_dispatch` publish path, two pre-existing
  Phase 20 review findings (`workflow_dispatch` triggering, `make publish-dry-run`), and the dead
  `upload_url` output in `scripts/create-or-reuse-release.sh`.

## Requirements

### Validated

**Security Tooling — shipped v0.9.0, 2026-09-01** (Phases 18-21, 25 plans, 20/20 requirements
verified). Archive: `.planning/milestones/v0.9.0-ROADMAP.md`. Requirements:
`.planning/milestones/v0.9.0-REQUIREMENTS.md`. Audit:
`.planning/milestones/v0.9.0-MILESTONE-AUDIT.md` (status `tech_debt`).

- ✓ A Rust SAST measured against a five-class planted-vulnerability probe before any adoption
  decision, with the disqualifying verdict recorded version-scoped and evidence-cited, the scan
  retained advisory-only behind a governed dismissal register, and every "no Rust SAST" record
  rewritten to the measured outcome (SAST-01 … SAST-04) — v0.9.0
- ✓ crates.io publishing authenticated per run via OIDC under a protected environment, the
  eleven-crate publish set reconciled (`paladin-herald` bootstrapped), the new path proven by a
  real publish before the old token was revoked and deleted in ratchet order, and the silent-skip
  green publish branch removed (PUB-01 … PUB-05) — v0.9.0
- ✓ A pre-publish consistency gate that blocks `cargo publish` until tag, manifests, changelogs
  and the tagged SHA's CI conclusion agree; same-tag re-runs as the supported recovery;
  registry-state already-published detection; per-crate outcome reporting with a no-crate-moved
  failure; and a rehearsed stuck-halfway runbook with a yank policy (PUBOPS-01 … PUBOPS-05)
  — v0.9.0
- ✓ Release artifacts made real and verifiable: curated-changelog release body with no git-log
  fallback, feature-correct binaries with existence asserts, digest-bound container image,
  aggregated checksums with verification instructions, stated SBOM scope, archived actions
  removed, all proven on a throwaway tag end-to-end (ARTIFACT-01 … ARTIFACT-06) — v0.9.0

**Milestone 2-12 close-out & Provider Expansion — shipped v0.8.0, 2026-08-24** (Phases 5-17,
149 plans, 65/65 requirements verified). Archive: `.planning/milestones/v0.8.0-ROADMAP.md`.
Requirements: `.planning/milestones/v0.8.0-REQUIREMENTS.md`. Audit:
`.planning/milestones/v0.8.0-MILESTONE-AUDIT.md` (status `tech_debt`).

- ✓ Milestone 2-3 recorded as 118 `file:line` verdicts, the three unverified blocks verdicted, and
  the Milestone 3 epic-numbering defect fixed at source (VERIFY-01 … VERIFY-06) — v0.8.0
- ✓ Grove routing honours its configured provider, with the no-fallback guarantee reachable from
  `GroveExecutionService::execute()` rather than only from an internal helper (CLOSE-01 … CLOSE-03)
  — v0.8.0
- ✓ The workspace's real shape recorded — ten library crates plus `doc-examples` plus the root
  facade — with four variant pairs and two policy questions answered, and every binary target given
  a documented purpose (ARCH-01 … ARCH-07) — v0.8.0
- ✓ Five verified defects closed: the broken API-surface CI job, missing deprecations, disabled
  `paladin-ports` doctests, leaked CLI dependencies, and duplicate `TokenUsage` collapsed to one
  canonical definition (DEBT-01 … DEBT-05) — v0.8.0
- ✓ Four divergent RustSec exception sets reconciled into one register with a mechanically enforcing
  guard, the licence posture settled, and the duplicate audit job deleted (SEC-01 … SEC-05,
  SUPPLY-01 … SUPPLY-03) — v0.8.0
- ✓ Milestones 7-8 and 9-12 recorded — 86 and 120 cited rows — with the 2026-06-04 reconciliation
  made authoritative and thirteen "never implement as written" entries made unmissable
  (HARD-01 … HARD-07, ORCH-01 … ORCH-05) — v0.8.0
- ✓ Every deferred item and removed feature given a decision rather than a rating, and the
  Milestone 9 candidate list triaged (FACADE-01 … FACADE-04) — v0.8.0
- ✓ The agent API's advertised capabilities made real — the token mechanism, the multi-replica
  store warning, and the LLM capability flag (WEB-01 … WEB-04) — v0.8.0
- ✓ The quality gates Deferred-QA Epic 25 specified and nobody started, now built and measuring:
  `coverage`, `cli-tests`, `bench-check`, `actionlint`, with the 82% floor single-sourced
  (PIPE-01 … PIPE-05, DEFER-01 … DEFER-03) — v0.8.0
- ✓ Milestone 11's documentation currency settled by content, and the 311-line architecture
  document dispositioned by ADR-0047 (DOCS-01 … DOCS-04) — v0.8.0
- ✓ Six new feature-gated LLM provider adapters — Kimi, Qwen, Grok, Ollama, Gemini and a generic
  operator-configured OpenAI-compatible provider — each meeting the full `LlmPort` contract, taking
  the shipped set from three providers to nine (PROV-01 … PROV-04) — v0.8.0
- ✓ Branch protection applied and verified live: three rulesets, `main` protected with 44 required
  contexts and no bypass on the merge gate, after a fast-forward reconciled a trunk 921 commits
  behind (Phase 15.1, SC1-SC7 — no REQ-IDs by recorded decision) — v0.8.0

**Milestone 1 close-out — shipped v0.7.1, 2026-08-04** (Phases 1-4, 38 plans, 25/25 requirements
verified). Archive: `.planning/milestones/v0.7.1-ROADMAP.md`. Audit:
`.planning/milestones/v0.7.1-MILESTONE-AUDIT.md`.

- ✓ Reconciled `.planning/` against shipped v0.7.0 code; cited status ledger covering every
  outstanding Milestone-1 task item with `file:line` verdicts (RECON-01, RECON-08) — v0.7.1
- ✓ One recorded answer per competing variant pair — `BattalionConfig`, `BattalionResult`,
  Formation minimum Paladin count, temperature range, Herald trait signature, coverage gate —
  each an evidence-cited ADR (RECON-02 … RECON-07) — v0.7.1
- ✓ Residual functional gaps closed — Chain of Command completion and tests, Battalion
  integration/performance tests, Herald on the Battalion execution path, Commander result
  normalization, the failing Auto-selection test, Garrison final validation, and the reconciled
  type definitions applied in code (GAP-01 … GAP-07) — v0.7.1
- ✓ Quality numbers made real — 85.92% workspace coverage against ADR-0006's 84% floor, 4 of 5
  zero-coverage first-party files closed, `#[ignore]`d Commander error-path tests activated, all
  five MCP failure modes tested, benchmarks re-enabled with a P50/P95/P99 baseline
  (QUAL-01 … QUAL-05) — v0.7.1
- ✓ Release coherence — version and edition agreement across all twelve manifests, advisory
  posture with written rationale, documentation review with a measured 15-minute quickstart, and
  the gate suite measured green at 2,924 tests / 185 doc tests / 47 example targets
  (REL-01 … REL-05) — v0.7.1

Prior shipped work, in the v0.7.0 workspace. Full per-requirement ledgers:
`.planning/REQUIREMENTS.md` →
*Milestone 1 as-shipped ledger* (115 IDs), *Milestone 2-3 as-shipped ledger* (118 IDs),
*Milestone 4-6 as-shipped ledger* (115 IDs), *Milestone 7-8 as-shipped ledger* (86 IDs) and
*Milestone 9-12 as-shipped ledger* (120 IDs) — **554 requirement IDs in total, all accounted
for.**

**Milestone 1 — the MVP framework** (confirmed by task-list checkbox state, 1,817 of 1,857 items,
and the codebase map):

- ✓ Paladin domain foundation — entity, builder, config, port, execution service with reasoning
  loop / retry / circuit breaker / stop words / timeout, error types, tracing, mock LLM (Epic 1)
- ✓ Garrison memory — entries, windowing and eviction, port + long-term port, in-memory and
  SQLite adapters, tokenizer-based counting, Paladin integration, config, errors (Epic 2)
- ✓ Arsenal tool system — Armament/Call/Result domain types, ports and registry, MCP client and
  transports, builder integration, timeout/concurrency controls, graceful degradation, context
  injection (Epic 3)
- ✓ Battalion orchestration — Formation, Phalanx, Campaign, error strategies, retry policy,
  status, logging, cancellation (Epic 4; Chain of Command verified in Phase 2 via GAP-01 —
  commander specialist selection, fallback survival and synthesized answer, with tests across
  all four delegation strategies)
- ✓ Commander strategy router — strategy enum, construction validation, Auto rule-based
  selection, unified `execute()`, error strategies, config passthrough, service composition
  (Epic 5)
- ✓ Provider expansion — capability-aware `LlmPort`, DeepSeek and Anthropic adapters, provider
  factory and per-Paladin selection, backward compatibility, error mapping, docs (Epic 6)
- ✓ Citadel state persistence — Paladin and Battalion state serialization, autosave, restore,
  checkpoint resumption, port and file adapter, state directory management (Epic 7)
- ✓ Herald output formatting — trait, JSON/Markdown/Table formatters, registry, streaming,
  configuration, default + per-execution override, error fallback (Epic 8)
- ✓ Armory CLI — `paladin agent|battalion|arsenal` command tree, YAML config schema, env-var
  API keys, validation and exit codes, output formatting, interactive prompts (Epic 9)
- ✓ Validation and documentation — integration test infrastructure, rustdoc and doc tests,
  24 user/technical docs, 22 examples, multi-arch Docker, Kubernetes manifests, GitHub Actions
  release and integration workflows (Epic 10) (**Amended by Phase 4, dated 2026-08-03, citing
  `04-release-measurement.md`**: "22 examples" restates the same Milestone 1 Epic 10 validation
  report amended above and in the Overview; the shipped tree carries 47 `.rs` files under
  `examples/`, 4 declared `[[example]]` targets, 0 crate-level `examples/` directories. See the
  Overview amendment for the full figure and precedence-order rationale.)

**Milestones 2-3 — the capability build-out and its completion** (component-level file evidence in
the tree, verified by direct inspection on `release/v0.7.0`; per-criterion confirmation is Phase 5):

- ✓ Sanctum long-term memory — `EmbeddingPort`, OpenAI embedding adapter, `SanctumPort`,
  in-memory and **Qdrant** adapters (`qdrant-client` 1.14, `qdrant` feature), Memory/MemoryType/
  decay domain model, configuration (Epics 11-12). The Epic 11 summary's "Qdrant DEFERRED" record
  is stale — `intel/code-verification.md` verifies it shipped.
- ✓ RAG pipeline — `RagRetrievalService`, `MemoryExtractionService`, `MemoryExtractionStrategy`,
  `RagConfig`, builder wiring (`with_sanctum`, `with_embedding_port`), context injection in the
  documented execution flow (Epic 12)
- ✓ Sentinel vision and documents — `VisionContent`/`ImageDetail`/`VisionRequest`/`VisionError`,
  OpenAI and Anthropic vision modules, **two coexisting ports** (`vision_llm_port.rs`,
  `vision_port.rs`), both Paladin entry points (`enable_vision`, `execute_with_vision`), PDF
  extractor, `DocumentPort`, CLI `--image`/`--document` (Epics 13, 20). The Milestone 3 release
  notes list this as unshipped Milestone-4 work; that is a stale forward-look.
- ✓ Autonomous agents — `MaxLoops::{Fixed, Auto}` enum, `PlanningService`, `TaskPlan`/`Subtask`,
  `PromptGenerationService`, `TemperatureService` with task-type bands, `HandoffService`,
  `HandoffStrategy`, the handoff tool, `AutonomousConfig` (Epics 14, 21)
- ✓ Conclave (mixture-of-agents) — domain model, parallel expert execution service with retry and
  partial success, Commander strategy, CLI/YAML, examples (Epic 15). Verified shipped against
  129 unchecked task items.
- ✓ Council and Grove — Council domain model with turn strategies and termination conditions plus
  its execution service; Grove with trees, tree agents, keyword/semantic/LLM routing and its
  execution service; both wired into Commander with examples and integration tests (Epic 16)
- ✓ Maneuver Flow DSL — grammar, lexer/parser/AST, `Maneuver` domain model, recursive execution
  service, ASCII and Mermaid visualizer, top-level CLI command group (Epic 17)
- ✓ CLI consolidation and enhancement — `src/cli` deleted and everything consolidated under
  `src/application/cli/`; `onboarding`, `setup-check`, `features`, `muster`, `council` and
  `maneuver` commands; rich formatters with `insta` snapshot tests (Epics 17.5, 18)
- ✓ Herald consolidation — placeholder types removed from `herald.rs`, real domain types imported,
  `TokenUsage` extracted to its own container (Epic 19)
- ✓ Battalion and Commander hardening — `PaladinRegistry` port and in-memory adapter,
  per-Paladin timing and token metrics on `BattalionMetadata`, Commander metadata export
  (Epic 22). One gap remains — see Active.
- ✓ CLI config and infrastructure completion — YAML garrison and arsenal/MCP configuration wired
  into `PaladinBuilder`, `MockLlmAdapter` and `MockArsenalPort`, three-tier test strategy,
  `SchedulerPort` with a `tokio-cron-scheduler` 0.13 adapter, content-deliverer scheduling
  (Epic 23). The most reliably complete epic in the run-2 corpus.
- ✓ Test hardening — benchmarks relocated into per-crate `benches/`, CLI snapshot suite, live-API
  test suite behind a feature flag (Epic 24)

**Milestones 4-6 — the refactor that restructured Milestones 1-3** (the best-evidenced block in
this planning set: 22 claims verified directly against `Cargo.toml` contents, type definitions and
file existence during ingest run 3, recorded in `intel/code-verification.md`):

- ✓ Feature-flag expansion — `default = ["llm-openai"]` replacing the old three-flag default;
  per-provider `llm-openai` / `llm-anthropic` / `llm-deepseek` / `llm-all`; subsystem flags
  `content-processing`, `web-server`, `notifications`, `vision`; a `full` convenience flag; and a
  `feature-flags.yml` CI matrix. The planned `mcp-arsenal` flag was **eliminated** by a dated PRD
  note and no MCP flag exists (Milestone 4 Epic 1)
- ✓ CLI isolation — a single `cli` feature gating the whole `src/application/cli/` tree, and
  `[[bin]] paladin-cli` with `required-features = ["cli"]`, plus a `cli_isolation` test target run
  in CI. Three CLI-only dependencies are still unconditional — see Active (Milestone 4 Epic 3)
- ✓ API-surface tooling — `scripts/{extract-public-api,check-api-surface,check-deprecations,check-all-examples}.sh`,
  `final-api.txt`, `api_surface_current.txt`, and the stable-API catalogue (now an mdbook chapter).
  The CI job that consumes them is broken — see Active (Milestone 4 Epic 2)
- ✓ Cargo workspace — root `[workspace] members = [".", "crates/*"]` with `resolver = "2"` and a
  full `[workspace.dependencies]` pin set; `paladin-core`, `paladin-ports`, `paladin-battalion`,
  `paladin-llm` and `paladin-memory` all extracted, `src/application/ports/` **fully deleted**
  (no shim), `paladin::prelude` shipped, and a `crate-isolation` CI job proving each crate builds
  alone (Milestone 5, all six epics)
- ✓ Upward-dependency resolution — `PaladinResult`, `StopReason`, `TokenUsage`, `RegistryError`
  and `HandoffError` moved into `paladin-core` with the ports reduced to re-exports, per the
  corpus's only Approved-status decision record. `PaladinError` was deliberately excluded
  (Milestone 5 Epic 1)
- ✓ Config decomposition — `application_settings.rs` **deleted** and replaced by per-domain
  modules split across the facade (`src/config/`), `paladin-memory` and `paladin-llm`, with an
  `EnvOverridable` trait and a `read_env` helper replacing ~30 copies of the env-override pattern
  (Milestone 6 Epic 1)
- ✓ Orchestration relocation — six manager-layer services moved out of
  `src/core/platform/manager/` and renamed to `*Orchestrator`, landing under
  `src/application/services/`; the manager module retains only `content_service`, `event_manager`
  and `user_service` (Milestone 6 Epic 2)
- ✓ Maneuver DSL co-location — the lexer, AST, parser, domain type, execution service and
  visualizer all consolidated under `crates/paladin-battalion/src/maneuver/`, and every parser
  reference removed from `paladin-core`. This **reverses** a Milestone 5 requirement that had just
  moved the parser into `paladin-core` (Milestone 6 Epic 3)
- ✓ `CircuitBreaker` relocation — moved to `src/infrastructure/resilience/`, with the old
  application-layer path **intentionally retired** and no re-export left behind. A `paladin-infra`
  crate and a `CircuitBreakerPort` trait were both explicitly rejected (Milestone 6 Epic 4)

**Milestone 7 — production hardening and the first published release** (verified against the tree
during ingest run 4):

- ✓ Four further crate extractions behind a written cost-benefit gate that returned **four Go, zero
  Defer** — `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`
  (Milestone 7 Epic 1)
- ✓ Production build infrastructure — `Dockerfile.chef` with a pinned `cargo-chef 0.1.77` and a
  workspace recipe, ten per-crate `make test-*` targets, a workspace feature-flag CI matrix, and a
  publish dry-run job (Milestone 7 Epic 2)
- ✓ Benchmark migration — all five suites moved into their owning crates with **zero `.disabled`
  files** left anywhere, three obsolete suites deprecated rather than restored, and a non-blocking
  `benchmark-regression-signal` CI job (Milestone 7 Epic 3)
- ✓ API stabilization through a real release — **`v0.1.0-rc.1` at commit `a9530fc`**, all ten crates
  published at `0.1.0` with a GO sign-off and docs.rs verification, and the crates.io collisions
  that forced the `paladin-ai` / `paladin-ai-core` package renames (Milestone 7 Epic 4).
  **This is history, not current state** — the tree is at `0.6.0` on `release/v0.7.0`

**Milestone 8 — facade cleanup, and a reconciliation that went further than the plan** (verified
during ingest run 4):

- ✓ 25 dead files deleted along with five orphaned directories; `src/core/` reduced to **exactly
  six files** (Milestone 8 Epic 2)
- ✓ `use_cases` → `services` renamed in **both** the facade and `paladin-content`, as a clean break
  with no compatibility alias — a workspace-wide grep for `use_cases` returns zero
  (Milestone 8 Epics 4 and 6)
- ✓ `paladin-web` consolidated on axum — actix-web removed entirely and **banned in `deny.toml`** —
  with the three delivery endpoints revived as mounted axum routes rather than deleted
  (Milestone 8 Epic 7)
- ✓ The 2026-06-04 reconciliation — **fifteen commits, ~10,250 net LOC removed, one new leaf
  crate** — which found the Epic 1 audit had mis-described ~4,400 LOC of orphaned uncompiled
  duplicates as "active bridges that stay", then executed the relocations Epic 3 had deferred to
  Milestone 9: `FileCitadel` → `paladin-memory`, MinIO/S3 and Redis → `paladin-storage`,
  `HashMapPaladinRegistry` → `paladin-battalion`, Herald formatters → the new `paladin-herald`

**Milestone 9 — classic orchestrator completion** (v0.3.0; verified against the tree during ingest
run 5, and the checkbox count of 0 open is corroborated):

- ✓ A real `execute_workflow()` at `src/application/services/orchestration/mod.rs:382`, covering
  all four `WorkflowExecutionOrder` variants and replacing four `println!`-only arms (Epic 1)
- ✓ `WorkflowRepository` output port plus a SQLite adapter with crash recovery —
  `crates/paladin-ports/src/output/workflow_repository_port.rs`,
  `crates/paladin-storage/src/sqlite_workflow_repository.rs`. Epic 1's Open Question 4 resolved by
  outcome: the adapter went to `paladin-storage`, its recorded default (Epic 1)
- ✓ Scheduler, queue and event operational validation, including the match / no-match / fan-out /
  rate-limit / trigger-to-dispatch tests against `ListenerOrchestrator` (Epic 2)
- ✓ Content processors in the **root crate** — `src/application/services/orchestration/processors/`
  — which is how Epic 3's circular-dependency Open Question actually resolved (Epic 3)
- ✓ The bidirectional content ↔ agent bridge: `OrchestratorPort` in `paladin-ports`,
  `OrchestratorBridgeAdapter` in the root crate. Its §6.1 is **the cleanest ADR-shaped section
  anywhere in the corpus** — a four-criterion comparison table, a `(CHOSEN)` column, an explicit
  decision, and the rejected option preserved as a future non-breaking enhancement (Epic 4)
- ✓ User/admin RBAC — `AuthPort` in `paladin-ports` with argon2 retained, an opaque hashed-token
  adapter in the root crate, axum auth middleware generic over `Arc<dyn AuthPort>` with RBAC tests
  (Epic 5). **Its §6.1 also recorded the trade-off that becomes WEB-02** — see Active

**Milestone 10 — CI hardening and release automation** (v0.4.0; every artefact verified, and **one
of its own acceptance criteria is false** — see Active):

- ✓ pre-commit framework, version-controlled, with a CI gate (`.pre-commit-config.yaml`,
  `.github/workflows/pre-commit.yml`, `make hooks`) (Epic 1)
- ✓ `cargo audit` reading `.cargo/audit.toml` as the single source of truth, `cargo deny check` as
  a required gate, OSV-Scanner annotate-only with SARIF upload, a CycloneDX SBOM in the release
  pipeline, `make security` (Epic 2)
- ✓ **Textbook licence-exception compliance** — the allow-list keeps six permissive licences plus
  four justified additions, each with an inline justification, and eight MPL-2.0 crates get
  per-crate `[[licenses.exceptions]]` entries rather than weakening the allow-list, on the
  reasoning that MPL-2.0 is *weak, file-level* copyleft (Epic 2 FR-14(b))
- ✓ `release.toml`, tag-triggered publishing, lockstep versioning, `make release` /
  `make publish-dry-run` / `make release-check` (Epic 3)
- ✓ Main-only tag enforcement **created in response to an incident** — the `verify-tag-source`
  guard with both workflow roots declaring `needs:` on it, plus committed GitHub rulesets for the
  main branch and release tags (Epic 5)

**Milestone 11 — documentation overhaul and publish** (v0.5.0; **92.0%, and its 26 open items are
the only genuinely open checkbox count in the entire corpus**):

- ✓ mdbook at `docs/` with mdbook-mermaid, the full nine-chapter hierarchy, GitHub Pages
  deployment and `docs/MIGRATION_LOG.md` — **the document that explains every "missing
  deliverable" runs 3 and 4 found** (Epics 1-2)
- ✓ Linkcheck as an **error**, not a warning: `warning-policy = "error"` with
  `follow-web-links = false`, after 227 broken links were repaired (Epic 3)
- ✓ All six deployment-topology pages — overview, embedded-library, battalion-orchestration,
  http-service-host, queue-worker, sidecar. **This Epic created Milestone 12**: writing the
  topology documentation surfaced a capability gap rather than papering over it (Epic 6)
- ✓ README rewritten as a concise landing page; version sync; v0.5.0 (Epics 5, 7)

**Milestone 12 — Web API** (v0.6.0; 99.0%, and all three open items are feature-branch scaffolding
while the code ships):

- ✓ Agent registry and execution API in `paladin-web`, behind the milestone's strongest
  architectural invariant — **`paladin-web` must not depend on the `paladin-ai` facade**, stated
  three times across two Epics with a mechanical verification command (Epic 1)
- ✓ Config-driven `paladin-server` binary: a consumer can start a Paladin agent HTTP service with
  only a `config.yml`, **writing no Rust** (Epic 2)
- ✓ SSE streaming, in-process async jobs, execution timeout and cancellation (Epic 3)
- ✓ Unified error envelope, health/ready endpoints, request logging with request IDs,
  CORS/body-limit/timeout layers, tower-governor rate limiting (Epic 4)
- ✓ API-key and bearer auth with constant-time comparison, per-agent `allowed_roles`, an admin gate
  on runtime registration, and a redaction test proving a key value does not leak (Epic 5).
  ~~**The bearer path is documented as JWT and implemented as opaque tokens**~~ — **resolved in
  Phase 14 (2026-08-12)**: the vocabulary was renamed to match the mechanism (ADR-0040), and the
  single-replica scope of the in-process store is now stated in the deployment artefacts (ADR-0041)
- ✓ OpenAPI generation with Swagger UI and a **committed `openapi.json` drift baseline** (Epic 6)
- ✓ `Dockerfile.server`, `docker-compose.yml`, `k8s/` manifests with liveness and readiness probes,
  a runnable `examples/http_service_host.rs`, and the deployment-topology docs updated to the
  shipped API — every pre-M12 disclaimer ("ships no agent-execution", "yours to compose") returns
  zero grep matches (Epic 7)

### Active

**Milestone v0.10.0 "Durable Agent Execution Runtime"** — scoped in `.planning/REQUIREMENTS.md`
as capability clusters over the `.project/v0.10.0/` PRD corpus (the PRDs remain the FR-level
source of truth). Eight categories, mirroring the epic structure plus program-level gates:

- [ ] **ENG-01 … ENG-08** — Battlefield typed state, superstep engine with cycles, Waypoint
  checkpointing/resume, three storage backends, legacy string bridge, engine seams, and the
  program scaffolding (`MIGRATION.md` skeleton, semver + MSRV CI jobs) mandated for the first
  epic by X-10.5/X-11.1 (Doc 01)
- [ ] **CF-01 … CF-05** — BUG-01 fail-closed custom edge conditions (the program's single
  sanctioned behavioral break), Directive routing, Muster fan-out, subgraphs, LLM routing (Doc 02)
- [ ] **HITL-01 … HITL-05** — Parley pause, validated resume, Chronicle history/replay/fork,
  graceful shutdown, minimal thread HTTP endpoints (Doc 03)
- [ ] **FT-01 … FT-06** — Transience taxonomy + structured NodeError, Aegis retry/timeout/error
  handlers, model fallback, node caching (Doc 04)
- [ ] **RT-01 … RT-06** — Middleware chain + built-ins, context management, Vault, structured
  output, provider-conformance close-out (Doc 05)
- [ ] **PLAT-01 … PLAT-06** — Background runs, worker pool + queue, parley/streaming integration,
  versioned assistants, schedules + webhooks, API cross-cutting + generated-client gate (Doc 06)
- [ ] **OBS-01 … OBS-04** — Trace event model + sinks, visualization export, eval harness (Doc 07)
- [ ] **SHIP-01 … SHIP-04** — `MIGRATION.md` complete, compat proofs (v0.9-config boot test,
  `openapi.json` golden diff), program acceptance audit, v0.10.0 release readiness (overview §5, §9)

*(The long-form forward-scope listing that previously lived here — the 90 ingest-derived
requirements across Phases 5-16 plus Phase 17's `PROV-*` additions — shipped with v0.8.0 and is
preserved verbatim in `.planning/milestones/v0.8.0-REQUIREMENTS.md` and this file's git history;
it is not restated because every item in it is either Validated above or recorded in the
archives.)*

### Out of Scope

- **Re-planning shipped Milestone 9-12 work** — the whole Milestone 9 orchestrator subsystem, the
  whole Milestone 10 tooling set, the mdbook and the whole Milestone 12 web API are **verified
  shipped against the tree** (37 rows). Phase 13 records them; no phase rebuilds them.
- **Treating Milestone 12's three open checkboxes or project-management's one as work** — the
  former are Task 0.0 feature-branch scaffolding while the Epic 5 code ships; the latter is a
  `- [ ] 1.1 Create template` formatting example inside a template file.
- **Re-ingesting `REQ-master-plan-epics-11-18` as new scope** — it is the *origin* document for
  Epics 11-18 (dated 2026-01-29), every one of which was ingested in run 2 and most of which are
  verified shipped. Its value is provenance, not scope.
- **Milestone 12's explicit non-goals** — hot-reloading `config.yml`, terminating TLS in
  `paladin-server` (a proxy/ingress concern), fine-grained scopes beyond `allowed_roles` plus the
  admin gate, and encrypting configuration at rest ("secrets management is the operator's
  responsibility, as with LLM keys").
- **Rewriting the 35 mdbook appendix files** — Milestone 11 Epic 3 non-goal. **One exception is
  under decision**: `design-and-architecture.md`, whose relocation into that exempt chapter is
  precisely why its gap survived (DOCS-02).
- **Benchmark regression *detection* (`critcmp`, `github-action-benchmark`)** — a Deferred-QA
  Epic 25 non-goal. Note the inversion: it already ships as `benchmark-regression-signal` from
  Milestone 7 Epic 3, while the `bench-check` compile prerequisite does not (PIPE-01).
- **Implementing the 14 requirements that shipped code superseded by outcome** — actix-web in
  `paladin-web`, the `storage-sqlite` flag, the per-crate ordered publish dry run, the `ml` feature
  gate, the Milestone 8 Epic 3 no-extraction mandate, the 160-file facade target (the tree reads
  136), and the root-path `STABLE_API.md` and `docs/*.md` deliverables the Milestone 11 overhaul
  relocated. Recording them is HARD-01; implementing them would undo shipped work.
- **Building `paladin-arsenal`, `paladin-sanctum` or `paladin-ml`** — none exists. The first two
  are named only by a superseded disposition record that contradicts its own governing PRD
  (FACADE-04 triages the list); the third is a *placement condition* on reintroducing a removed
  feature (FACADE-03), not a deliverable.
- **Treating any `v0.1.0-rc.1` artefact as current state** — the published-crate list, the docs.rs
  verification and the GO sign-off all describe `0.1.0`. HARD-03 records the trajectory as history.
- **Re-planning shipped work** — Milestone 1 is 98% checked, Milestones 2-3 are shipped wholesale,
  and Milestones 4-6 are verified shipped against the tree. Anything already satisfied by code is
  recorded in a ledger, not re-planned as a phase. That explicitly includes the entire workspace
  decomposition and all four Milestone 6 relocations.
- **Converting open checkbox counts into requirements** — 542 items are unchecked, and the two
  largest blocks (Conclave 129, Sanctum 111) are verified shipped. Verification precedes planning.
- **Picking winners among the 30 competing variant groups (69 warnings)** — recording answers is in scope;
  choosing inside an ingest artefact is not. Explicitly requested: variants are expected and
  settling past disagreements is not the goal of this ingest. Where shipped code settles a variant,
  that is recorded as a **fact about the tree** at the top of the precedence order, not as a
  decision taken in a planning file.
- **Promoting the two ADR candidates into locked decisions** — doing so requires re-tagging the
  source documents via `--manifest` and re-running ingest, not an edit here. See Key Decisions.
  (**Corrected by Phase 12 (plan 12-03), dated 2026-08-09, citing `.planning/decisions/PROMOTION.md`
  §Part A and `.planning/decisions/0036-audit-suppression-single-source-topology.md` (ADR-0036):**
  the `--manifest` requirement is superseded — `PROMOTION.md` §Part A states promotion is now an
  ordinary write to a directory plus a table row, since ADRs live in `.planning/decisions/` as their
  own document class, independent of the ingest manifest, and top the precedence order. Both counts
  above are stale: this is no longer "the two ADR candidates," and the promotion this bullet declared
  out of scope has now happened for one of them — candidate 7
  (`Milestone_10/Epic_2/prd-dependency-security-license-compliance.md` FR-1 + §8) is closed by
  ADR-0036, `Accepted`, `conforms`. See the `## Key Decisions` row plan 12-04 adds.)
- **Building `STABLE_API.md`, `docs/FEATURE_FLAGS.md`, `docs/MIGRATION.md` or
  `docs/CONFIGURATION.md`** — absent from the paths six run-3 documents name, but shipping as
  mdbook chapters under `docs/src/`. Recording the relocation is ARCH-05.
- **Migrating between the two shipped vision port surfaces** — both ship deliberately;
  `code-verification.md` says confirm intent before planning a migration.
- **Decomposing the three oversized service files** (2,757 / 2,294 / 1,840 lines) — real tech
  debt, but no ingested requirement demands it. Tracked as v2.
- **Clone/lock-contention performance work** — the 383 `.clone()` calls and the 9 orchestrator
  locks are flagged in `codebase/CONCERNS.md`, but optimizing before Phase 3 restores
  benchmark baselines would be guesswork. Tracked as v2.

## Context

**Current state stamp (v0.9.0 close, 2026-09-01):** 4 shipped planning milestones (v0.7.1,
v0.8.0, v0.9.0 on top of the twelve historical ones), 21 phases / 212 plans completed, ~142k
lines of Rust across 385 first-party `.rs` files, eleven publishable crates. Supply-chain
posture: no long-lived publish credential, OIDC-only publishing, gated + idempotent + rehearsed
release pipeline, advisory-only CodeQL with the manual credential review as primary control.

**THE SINGLE MOST IMPORTANT FACT ABOUT THIS CORPUS: nothing in it is locked.** Across all **263**
documents — five ingest runs, twelve milestones, 554 requirements, eighteen months of planning —
there are **zero ADR-typed and zero SPEC-typed documents.** Not one. **Every technical decision in
twelve milestones sits at PRD or DOC precedence and is auto-overridable by any document that
arrives later.**

That is not an artefact of the classifier: every one of the 199 classifications carried
`manifest_override: true` and `confidence: high`, and no document anywhere carried `locked: true`.
It has three consequences worth stating plainly:

1. **No LOCKED-vs-LOCKED contradiction was ever possible**, in any run. That is why the corpus has
   0 blockers across five runs despite 69 competing variants — there was never a pair of protected
   positions that could collide.
2. **Mechanical precedence has already produced at least one architecturally wrong answer.** The
   corpus's only Approved-status decision record is manifest-typed DOC, so a PRD published two days
   later outranks it — and that PRD's rule would pull `PaladinResult`, `StopReason` and `TokenUsage`
   back out of `paladin-core` and reintroduce the exact upward dependency the decision removed
   (ARCH-03(c), variant group 19).
3. **The project's real arbiter is the shipped tree, by necessity rather than by preference.**
   Precedence here runs **ADR → shipped tree → `.planning/codebase/` map →
   `intel/code-verification.md` → PRD → DOC → task-list checkbox**, and checkbox state sits last
   because five runs found it wrong in both directions.

**Eleven ADR candidates have accumulated, and none is promoted.** Promoting any requires re-tagging
its source via `--manifest` and re-running ingest — entering one in Key Decisions would fabricate
authority the corpus does not contain. In rough order of consequence:

(**Corrected by Phase 12 (plan 12-03), dated 2026-08-09, citing `.planning/decisions/PROMOTION.md`
§Part A and ADR-0016, ADR-0021, ADR-0024, ADR-0025, ADR-0036:** all three claims in this passage are
stale. **Not "none is promoted":** four of the eleven are now promoted — candidate 1 by ADR-0016,
candidate 2 by ADR-0021, candidate 3 by ADR-0024, candidate 5 by ADR-0025 — and this phase adds a
fifth: candidate 7, item 2 below, is closed by ADR-0036. **The `--manifest`/re-ingest requirement is
superseded:** `PROMOTION.md` §Part A states promotion is now an ordinary write to a directory plus a
table row, since ADRs live in `.planning/decisions/` as their own document class, independent of
the ingest manifest, and top the precedence order. **Item 2's "Currently violated in the tree" is
no longer true:** ADR-0036's `## Code Conformance` section carries the `conforms` verdict and the
measurement establishing it, not restated here.)

1. **`Milestone_7/Epic_4/rustsec-remediation-plan.md`** (run 4) — a formal risk acceptance with
   **owner Platform Security** and **review/expiry target 2026-09-30**. **The only item in all 263
   documents carrying an expiry date**, and the only candidate where not promoting it has an
   ongoing operational cost.
2. **`Milestone_10/Epic_2/prd-dependency-security-license-compliance.md`** (run 5) — the
   audit-suppression single-source invariant, FR-1 + §8. **Currently violated in the tree.**
   *(1 and 2 are the same subject from two milestones. Promoting them together would turn the
   run-5 supply-chain finding from an observation into a gate — SUPPLY-03.)*
3. **`Milestone_5/Epic_1/decisions/battalion-result-upward-dependency-decision.md`** (run 3) — the
   only decision/options pair in all 263 documents, with `Status: Approved`,
   `Decision Date: 2026-05-13`, `Chosen Option: Option A`, a Rationale, a Rejected Options section
   and an implementation checklist. Shipped code implements it, and a PRD outranks it.
4. **`Milestone_9/Epic_5/prd-user-admin-system-completion.md` §6.1** (run 5) — the opaque-bearer-token
   decision, with rationale, a rejected alternative and a recorded trade-off. **The only decision in
   the corpus that a later milestone contradicts in prose while silently preserving in code**
   (WEB-01).
5. **`Milestone_9/Epic_4/prd-agent-orchestrator-bridge.md` §6.1** (run 5) — the cleanest ADR-shaped
   section anywhere: a four-criterion comparison table, a `(CHOSEN)` column header, an explicit
   decision, and the rejected option preserved as a future non-breaking enhancement.
6. **`Milestone_7/Epic_1/cost-benefit-assessment.md`** (run 4) — a go/defer record with a
   "Self-Approval (Task 1.6)" block, a named approver and an approval date; its governing PRD calls
   it "the authoritative source of record for *why* a decision was made".
7. **`Milestone_7/Epic_4/license-compatibility-decision-checklist.md`** (run 4) — a licensing policy
   with approver `DF3NDR`, approval date 2026-05-28 and a 551-package inventory. Contradicted by
   the shipped manifests → SEC-02.
8. **`Milestone_8/facade-cleanup-RECONCILIATION-2026-06-04.md`** (run 4) — an explicit supersession
   notice carrying `Supersedes (corrects):` two named documents, which then resolved all six of its
   own listed open decisions **in execution rather than by a recorded decision**.
9. **`Epic_17.5/epic17-5.md`** (run 2) — the CLI belongs in `src/application/cli` because "CLI is an
   input adapter in the application layer, not infrastructure". Already applied in code, also
   outranked by a PRD that says otherwise.
10-11. Two further run-5 candidates of lower consequence, listed in `intel/decisions.md`.

**Where the code actually is.** The committed codebase map (`.planning/codebase/`, refreshed
2026-07-30) plus `.planning/intel/code-verification.md` are authoritative on current state, and
both are *ahead of* the ingested documents in several places:

- MCP is implemented on the official `rmcp` 2.1.0 SDK with STDIO and **Streamable-HTTP**
  transports — the Milestone-1 PRD specified a hand-rolled client with an **SSE** transport.
- Herald is already wired into `PaladinExecutionService` (`with_herald`, used in the execution
  path) and `chain_of_command_service.rs` exists — both were listed as incomplete in the
  January task lists. **The task lists are a point-in-time snapshot; the code is the arbiter.**
- Epic 9 declared "no REPL or interactive shell" a non-goal; an interactive REPL now ships. A
  documented non-goal has already been superseded by later work — which is exactly why nothing in
  this file is treated as locked.
- Conclave, Council, Grove, the Maneuver Flow DSL, Sentinel vision and the Qdrant Sanctum adapter
  are all **verified shipped** despite documents that variously declare them deferred, unstarted,
  or scheduled for Milestone 4.

**Milestones 4-6 restructured everything Milestones 1-3 built — and it is all in the tree.**
Ingest run 3 is the first run where documents and code mostly *agree*. Verified directly:
`[workspace] members = [".", "crates/*"]` in the root `Cargo.toml`; `src/application/ports/` gone
entirely (full deletion, not a shim); `application_settings.rs` gone, replaced by
`src/config/{agents,arsenal,citadel,env_utils,file_storage,herald,notifications,queue,scheduler,settings,web_server}.rs`
plus `crates/paladin-memory/src/config/` and `crates/paladin-llm/src/config/`;
`src/application/use_cases/` gone entirely, with the orchestrators under
`src/application/services/`; `crates/paladin-battalion/src/maneuver/` holding the whole Flow DSL;
`src/infrastructure/resilience/circuit_breaker.rs`; `src/prelude.rs`; and the `crate-isolation`
job at `ci.yml:228`. **Relocation is not contradiction** — the supersession chains are recorded in
`REQUIREMENTS.md`.

**Five documented positions are contradicted by shipped code and must not be applied literally.**
`vision` gating `chacha20poly1305` and `zeroize` (shipped `vision = []` gates nothing; the two
crates serve user auth and Citadel encryption, so gating them would break
`cargo build --no-default-features` — the epic's own dependency-matrix audit said so and the PRD
was wrong); the MCP transport feature flags (none exist; the PRD's dated elimination note is what
shipped); `web-server` gating `actix-web` (shipped as axum only); a `paladin-cli` workspace crate
(never built — the CLI is a feature plus a binary target); and `src/application/use_cases/` as the
orchestration home (shipped under `src/application/services/`). Correcting these at source is
ARCH-05.

**Run 3 is the first run where a checkbox count proved trustworthy — and the first where one
overstated completion.** Milestone 4's 20 open items are real: `grep -rn '#\[deprecated' src crates`
returns 0, and `DEPRECATIONS.md` agrees. Milestone 6's 0 open items are real: all four relocations
are complete. But Milestone 4 Epic 3's task list is fully checked while three CLI-only
dependencies remain unconditional. The lesson is not "checkboxes understate" — it is
**"verify each count against the tree"**.

**Five verified open defects, all small.** The `api-surface` CI job fails on every run because
`ci.yml:171,181,186` and both scripts point at `project/current-exports.txt` while the file lives
at `.project/current-exports.txt` after commit `928c6d5` — so the project's only automated
public-API guard has been inert. Zero `#[deprecated]` annotations exist against Milestone 4 Epic 2
FR-8. `paladin-ports` sets `[lib] doctest = false` deferring the fix to an unwritten "Task 7.0",
and CI excludes the crate from `--doc`. Three CLI-only dependencies still compile into library
builds. Three `TokenUsage` structs ship where the decision record names one. These are Phase 8.

**The corpus now has a second ADR candidate, and it is stronger than the first.**
`Milestone_5/Epic_1/decisions/battalion-result-upward-dependency-decision.md` carries
`Status: Approved`, `Decision Date: 2026-05-13`, `Chosen Option: Option A`, a Rationale, a Rejected
Options section and an implementation checklist, with a full three-option trade-off analysis in its
`-options.md` sibling. It is the only decision/options pair in all 263 documents. It is
nevertheless manifest-typed **DOC**, so it sits at the lowest precedence tier and a PRD published
two days later contradicts it — which means mechanical precedence would pull `PaladinResult`,
`StopReason` and `TokenUsage` back out of `paladin-core` and reintroduce the exact upward
dependency the decision removed. **Two caveats on the record, both important:** it settles the
*location* of five value/error types and nothing else, and despite its filename it **never
mentions `BattalionResult`** — the run-1 `BattalionResult` variant is closed by shipped code, not
by this document.

**Two run-1/run-2 questions were closed by run-3 code verification.** `BattalionResult`'s field set
resolves to a merged superset at `battalion/mod.rs:549` that satisfies all three consumers, so
RECON-03 became a recording task and GAP-07 lost its code change. `BattalionConfig` resolves to the
Epic 4 form exactly, and `CommanderConfig` — the third claimed owner of `metadata_output_dir` —
does not exist anywhere in the tree. The competing `ErrorStrategy` variant sets turned out to be
two distinct enums in two different crates, which Milestone 6 physically separated.

**Milestone 7-8 is the first block where a document audits itself against the tree — and it is the
most reliable thing in the corpus.** `facade-cleanup-RECONCILIATION-2026-06-04.md` re-audited `src/`
file by file, found that the Epic 1 audit and the Epic 3 disposition record had described ~4,400 LOC
of *orphaned, uncompiled duplicate files* as "active bridges that stay" ("they are not bridges; they
are dead corpses left behind when the real code was copied into leaf crates"), and then executed in
fifteen commits the relocations Epic 3 had deferred to Milestone 9 — creating `paladin-herald`
inside an Epic whose §5 non-goals state "No new crates created. `paladin-herald`, `paladin-ml`, etc.
are not in scope". Its verification method is stated and reproducible, and the tree confirms every
target. **Three of its in-execution corrections matter more than the deletions**: `paladin_registry.rs`
was *not* a duplicate (the facade's 418-LOC impl was richer than battalion's 67-LOC `pub(crate)`
copy, so the richer one was consolidated *into* battalion); `sqlite_*_repository.rs` were *not*
redundant (they were the active default-build impl, resolved by making `paladin-storage`
non-optional); everything else genuinely was orphaned. **Recording this as the authoritative account
of Milestone 8 is HARD-02** — and the reason the earlier "9 crates" figure was wrong.

**Two Milestone 8 epics are complete despite their own records saying otherwise.** Epic 6 is filed
"Not verified; low priority" by the reconciliation and omitted from `deferred-items.md`, yet
`crates/paladin-content/src/services/` ships, `lib.rs` declares `pub mod services;`, and a
workspace-wide grep for `use_cases` returns zero. Epic 3 is filed "PUNTED" and is complete in
substance. Both of Milestone 8's three open checkboxes are contradicted by code — the same pattern
runs 1-2 found for Conclave and Sanctum.

**The security gates do not hold, and one of them has a deadline.** Four surfaces encode four
different RustSec exception sets: `rustsec-remediation-plan.md` formally risk-accepts **two**
advisories with owner Platform Security and **review/expiry target 2026-09-30**; `.cargo/audit.toml`
suppresses **five**; `deny.toml` suppresses **fifteen** while its own header claims to mirror
`audit.toml` and instructs "keep these two files in sync"; and `ci.yml` runs **two independent
`cargo audit` jobs** — a bare one at `:77` reading `audit.toml`'s five, and one at `:406` passing
the original two inline. Thirteen of `deny.toml`'s fifteen have no entry in the formal
risk-acceptance register; they carry inline one-line reasoning but no owner and no expiry, against
acceptance criteria that require both. Both `cargo audit` and `cargo deny` gate CI. **2026-09-30 is
the only date anywhere in the 153-document corpus**, and nothing in `.planning/` other than SEC-01
surfaces it.

**Four small verified defects sit alongside those gates.** `paladin-herald` has a `README.md` but no
`CHANGELOG.md`, against a criterion the Epic 4 completion summary records as Met (the crate was
created after Epic 4 closed). `Dockerfile.chef` enumerates nine crate manifests in its planner stage
and omits the tenth, so the cache-tightness FR-01 exists to deliver is not achieved for herald. The
`api-surface` baseline path is unchanged since run 3 — and is now **written into a run-4
requirement**, M8 Epic 7 FR-10, so DEBT-01 must fix the requirement as well as the tooling.
`paladin-ports` doctests remain disabled behind the same unwritten "Task 7.0".

**Two architecture questions are worth surfacing rather than assuming.** The extracted-crate
dependency rule is stated absolutely — "No extracted crate may depend on another extracted crate" —
and violated exactly once, by `paladin-content`'s optional `paladin-llm` edge, which the same PRD's
own complexity assessment anticipated without amending the rule (HARD-05). And `pdf = []` in
`paladin-content` gates no dependency while the facade's `content-processing` omits it entirely —
yet `.cargo/audit.toml` suppresses an advisory on the stated grounds that `pdf-extract` *is* in the
graph (HARD-06). The second blocks an honest reconciliation of the first set of advisories.

**The version trajectory is history, and must not be mistaken for state.** Milestone 7 Epic 4 cut
**`v0.1.0-rc.1`** at commit `a9530fc` on 2026-05-28 — all ten crates published at `0.1.0`, every
release gate PASS, a GO sign-off, docs.rs verification for all ten, and an external smoke project
compiling against `paladin-ai = "0.1.0"`. Its own PRD had targeted lockstep `0.2.0`. Milestone 8
targeted v0.2.0; its Epic 7, written 2026-06-06, targets "post-v0.5.1", so v0.3.0 through v0.5.1 all
shipped in between. The tree is at `0.6.0` on `release/v0.7.0` with latest tag `v0.5.1`. HARD-03
records the trajectory; REL-01 converges the three-way disagreement and must not converge on an rc.1
figure.

**The trustworthy remaining-work signal is the deferred registers, not checkbox arithmetic.**
`deferred-items.md` (D1-D5) and `deferred-features.md` (the `paladin user` CLI surface and the
TensorFlow adapter) are verified exact against the tree — D5's claim of 17
`println!`/`eprintln!`/`dbg!` occurrences across 6 files matches to the occurrence, and `src/core/`'s
six files, the three mis-layered manager services and both feature removals all check out. The
`deferred-features.md` TensorFlow entry carries the load-bearing constraint: any future ML adapter
must live in a dedicated `paladin-ml` **leaf crate**, never the facade — the surviving half of the
non-goal that `paladin-herald` overrode.

**The precedence order this project uses**, most authoritative first:
**ADR → shipped tree → `.planning/codebase/` map → `intel/code-verification.md` → PRD → DOC →
task-list checkbox.** Three ingest runs have now found checkbox state wrong in both directions, so
it sits last by evidence rather than by preference. An ADR that contradicts shipped code is an
instruction to change the code, not a description of it — that is why every ADR must carry a
`Code conformance` field (`conforms` or `must change`, naming the executing requirement in the
latter case): "authoritative" and "already true" are not the same claim, and the field is what
keeps them from being confused (D-02, D-03; see `.planning/decisions/`).

**Two coexisting vision surfaces, on purpose.** Epic 13's `VisionCapableLlm` lineage
(`crates/paladin-ports/src/output/vision_llm_port.rs`, reached via
`PaladinBuilder::enable_vision`) and Epic 20's `VisionPort` lineage (`vision_port.rs`, reached via
`PaladinExecutionService::execute_with_vision`) both ship. The ingest report preserved these as
competing variants; `code-verification.md` overrides that — they are two coexisting ports, not an
unresolved contradiction. Confirm intent before planning any migration (VERIFY-04).

**One documentation defect is propagating through the corpus.**
`RELEASE_NOTES_MILESTONE_3.md` numbers Milestone 3 Epics 19-23 as Conclave / Council / Grove /
Maneuver / Commander Enhancement. Those four patterns are Milestone **2** features (Epics 15, 16,
16, 17), all verified shipped. The Milestone 3 plan, all six epic definitions, every PRD and every
task list instead use 19 = Herald consolidation, 20 = Vision, 21 = Autonomous, 22 = Battalion
hardening, 23 = CLI/Config, 24 = Test hardening. **The plan/epic-definition numbering is
authoritative** (8 of 9 documents plus all task lists) and is the only mapping used in
`ROADMAP.md` and `REQUIREMENTS.md`. Four further documents mislabel epic numbers in
cross-references. Fixing this at the source is VERIFY-03.

**Measured quality state.** Milestone 1 (Epic 10 Task 6.0, 2026-01-27): 1,091 tests passing / 0
failures (706 unit, 385 integration, 133 doc); `cargo fmt --check` clean; `cargo clippy -- -D
warnings` at 0 warnings after fixing 102 across 48 files; unit coverage 60.88%, integration 67.79%;
2 medium transitive advisories; 22/22 examples compiling; Docker image 112 MB built in 5m31s; all
benchmarks disabled. (**Provenance note added by Phase 4, dated 2026-08-03, citing
`04-release-measurement.md`**: this "22/22 examples compiling" line is the historical origin of
the "22 examples" figure restated elsewhere in this document, in `ROADMAP.md` and in
`REQUIREMENTS.md` — all four of those restatements are amended at source with the same date. This
line itself is **not** corrected: it is Milestone 1's own dated measurement of the tree as it
existed on 2026-01-27, and a historical measurement is not stale merely because the tree has since
grown to 47 example files. Phase 4's re-derivation measures the *current* shipped tree, not this
one.) Milestone 3 release notes report ~78% overall coverage, Battalion
orchestration overhead < 10 ms for 100+ concurrent battalions, Garrison queries < 50 ms on a
1,000-entry store, and Herald formatting at 0.0095 ms for a 10 KB result. Reported test totals
across the corpus (999 → 1,292 → 1,674 → 1,628 → 853) are not monotonic and none is treated as
authoritative.

**Version state is incoherent right now.** Branch `release/v0.7.0`, workspace `Cargo.toml`
version `0.6.0`, latest tag `v0.5.1`. Three different answers to "what version is this".
Phase 4 resolves it.

**Milestone 9-12 landed, and run 5 is the run that found the most genuinely unbuilt work.** Not
because those milestones underdelivered — M9 100%, M10 100%, M11 92.0%, M12 99.0%, all corroborated
or exceeded by the tree — but because run 5 is the first to ingest a register whose work was never
started. **`Deferred-QA-CICD-Completion` Epics 25-27 were verified open item by item**, not
inferred: no `cli-tests` job, no `bench-check` job, no `coverage` job, no `.codecov.yml`, no
Makefile coverage targets, eight deprecated GitHub Actions, an architecture document frozen at
exactly 311 lines, an empty `docs/assets/`, no `docs/DEMOS.md`, and no `tools` field,
`ToolDefinition` or `ToolCall` anywhere in the workspace.

**A completed milestone's own acceptance criterion is false, and the mechanism is known.**
`ci.yml` contains **two jobs with the identical display name `Security Audit`**. The one at
`:60-77` runs a bare `cargo audit` under a comment declaring `.cargo/audit.toml` the single source
of truth — compliant. The one at `:389-406` runs
`cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2025-0111` — covering 2 of the 5
advisories in that file. Since `cargo audit` scans `Cargo.lock` irrespective of feature selection,
**the two jobs are configured to reach different verdicts on the same tree.** The Epic 25 PRD's
Appendix B tabulates the pre-Milestone-10 pipeline as 7 jobs, of which #4 is `security`: Milestone
10 Epic 2 **added** the compliant job without removing its predecessor, and Epic 4's non-goals then
froze the area ("No changes to `deny.toml` or `.cargo/audit.toml`", "No new CI jobs"). Milestone 10
is recorded **100% complete with 0 open checkboxes** and its Epic 2 §8 metric — "no inline
advisory-ignore flags remain in CI" — is false. **The fix is deleting 18 lines** (SUPPLY-01).
(**Corrected by Phase 12 (plan 12-01), dated 2026-08-09, citing `ci.yml:465-482` and commit
`cb75b2b`:** the `:389-406` citation two paragraphs above never held this job — it was re-derived at
`ci.yml:465-482` and deleted by Phase 9's plan 09-06 in commit `cb75b2b`, so the citation was
already stale before Phase 9 touched anything. The job no longer exists and the fix is done — see
SUPPLY-01's "Verified by Phase 12" closure block in `REQUIREMENTS.md`.)

**A run-4 finding is corrected, not extended.** Run 4 recorded `deny.toml` as out of sync with
`.cargo/audit.toml` — mirroring "only the original two" advisories while the three 2026 advisories
were "absent". **That is no longer true and the framing is withdrawn.** Both files carry the same
**five** vulnerability advisories; `deny.toml`'s ten additional entries are *unmaintained /
maintenance-mode* notices, a different advisory class, filed under a header that says so and
explicitly authorised by Milestone 10 Epic 4 FR-1 step 5. **The real gap is narrower: 13 of the 15
suppressions carry documented reasoning but no named owner and no expiry**, against a Milestone 10
Epic 2 origin policy that mandates a single documented exception process — and Epic 2 FR-3's
four-field schema does not require an owner or an expiry either, so the configs are compliant with
their policy and **the policy is the gap.** Separately, the three 2026 *vulnerability* ignores are
authorised by **no** ingested document: FR-3 and §5 name exactly two. SUPPLY-02.

**The agent API is documented as JWT and implemented as opaque tokens.**
> **RESOLVED in Phase 14 (2026-08-12).** The analysis below records the state as found. Phase 14
> renamed the vocabulary to match the mechanism rather than changing the mechanism: see ADR-0040
> (opaque bearer-token mechanism ratified) and ADR-0041 (in-process store, single-replica scope,
> shared store deferred with a named trigger). Open Question 4 is dissolved, not answered.
`grep -rn "jsonwebtoken" Cargo.toml crates/*/Cargo.toml` returns **nothing** — the crate is not a
dependency anywhere in the workspace. The only `AuthPort` implementation is
`src/infrastructure/adapters/auth/in_memory_token_auth_adapter.rs`, Milestone 9 Epic 5's opaque,
in-process, hashed-token store, chosen deliberately with JWT listed as an explicit non-goal. Yet
`crates/paladin-web/src/agent_auth.rs` documents its verifier as JWT throughout — module docs, the
`jwt: Option<Arc<dyn AuthPort>>` field, the "bearer JWT checked first" comment — and Milestone 12
Epic 5's **Open Question 4** ("which concrete `AuthPort` impl does `paladin-server` wire, and what
signing secret/algorithm does it need?") is unanswered *because it is unanswerable for the shipped
adapter*. **This is the only variant in five runs that shipped code cannot settle**: the tree
carries the Milestone 12 *shape* and the Milestone 9 *mechanism* simultaneously (WEB-01).

**And it has an operational edge.** Milestone 9 Epic 5 §6.1 recorded the trade-off in its own
words — "tokens are validated against an in-process store, so a **multi-process deployment would
later need a shared store**" — and Milestone 12 Epic 7 then shipped `k8s/deployment.yaml`, whose
purpose is multi-process serving. **Under more than one replica, a token issued by one pod will not
verify on another.** Neither document references the other, and no requirement in the 263-document
corpus covers the shared store Milestone 9 anticipated. This is a correctness question, not a
scaling optimisation (WEB-02).

**The corpus's largest documentation gap was hidden by a relocation.**
`docs/src/appendix/design-and-architecture.md` is **exactly 311 lines** — the identical figure its
own February 2026 PRD cites as the *pre-rewrite* state. Whole-word case-insensitive counts in that
file: **Commander 0, Council 0, Conclave 0, Grove 0, Maneuver 0, Sanctum 0, Sentinel 0**; zero
```mermaid blocks. All seven subsystems are verified shipped in the tree. Milestone 11 Epic 2's
appendix escape hatch ("docs with no single-chapter home are placed in an `appendix/` chapter
rather than dropped") moved the file, and Milestone 11 Epic 3's non-goals then exempted exactly
that chapter from rewriting ("the 35 appendix files are reference/archive material and are not
rewritten in this Epic"). **The relocation placed the gap into the one chapter nobody was required
to fix, where it has been invisible for two milestones** (DOCS-02).

**One checkbox count out of 542 survives verification.** Five runs establish the pattern
conclusively: counts understated shipped reality (runs 1-2: Conclave 129 open and shipped, Sanctum
111 open and shipped), were accurate once and overstated once (run 3), were contradicted outright
(run 4: Milestone 8's three), and were vacuous or nonexistent (run 5: Milestone 12's three are
Task 0.0 feature-branch scaffolding while the Epic 5 code ships; project-management's one is a
`- [ ] 1.1 Create template` formatting example inside a template file). **Only Milestone 11's 26
content-currency items are genuine** — and even those are "update in-place" tasks against fourteen
files that all exist, so they must be settled by content rather than by presence (DOCS-01).

**Two registers propose incompatible next actions on the same file.** Deferred-QA Epic 28 plans to
*test* `src/core/platform/manager/user_service.rs` to ≥ 80% (488 LOC, ~4.23% coverage, 15-20 h);
Milestone 8's `deferred-items.md` D2 plans to *split* it, because it is mis-layered. Run 4
established `deferred-items.md` as the highest-fidelity document in the corpus, so D2 carries real
weight. Splitting first and testing the resulting units is cheaper, but it changes Epic 28's
estimate and its mock set. **Sequence them deliberately** (FACADE-02 ↔ DEFER-02).

**The third deferred register is materially less reliable than the first two.**
`DEFERRED_COVERAGE.md` and the Deferred-QA PRDs carry a named sign-off ("AI Coding Agent (Epic 24
execution), February 14, 2026") and a "Next Review: Epic 27 or Epic 28 planning" trigger that was
never reached. But **both of its module paths are stale** — one relocated by Milestone 6, one still
present — and **its coverage baselines predate Milestone 9's tests on the same modules**. Treat its
*scope* as real and its *numbers* as needing re-measurement (DEFER-03).

**Nine stale references, unchanged across three ingest runs.** `project/current-exports.txt` was
renamed to `.project/` in commit `928c6d5`. Run 3 found five references (two scripts, three `ci.yml`
lines) and established that `check-api-surface.sh` exits 1 with "No baseline found" when the file
is absent, so the `api-surface` CI job fails on **every** run. Run 4 added a sixth, inside a
requirement. Run 5 adds **four more — all Milestone 12, written in June 2026, months after the
rename.** The newest requirements in the corpus are propagating the defect forward. It is the
longest-lived unfixed defect here and the cheapest to close (DEBT-01).

**Ingest program — COMPLETE.** This planning setup was bootstrapped from `.project/Milestone_1-MVP`
(36 docs) in run **1 of 5**; merged `.project/Milestone_2-Missing_features` +
`.project/Milestone_3-Completion` (45 docs) in run **2**; merged
`.project/Milestone_4-Refactor-Crates-Features` + `.project/Milestone_5-Workspace-Decomposition` +
`.project/Milestone_6-Architectural-Refinements` (32 docs) in run **3**; merged
`.project/Milestone_7-Production-Hardening` +
`.project/Milestone_8-Facade-Cleanup-Shim-Resolution` (40 docs) in run **4**; and merged
`.project/Milestone_9-Classic-Orchestrator-Completion` +
`.project/Milestone_10-CI-Hardening-Release-Automation` +
`.project/Milestone_11-Documentation-Overhaul-Publish` + `.project/Milestone_12-Web-API` +
`.project/Deferred-QA-CICD-Completion` + `.project/project-management` (46 docs) in run **5 of 5 —
the final run**.

**Corpus coverage reconciles exactly.** `.project/` holds **263** `.md` files: **188** prose /
planning documents and **75** `tasks-*.md` checklists. The five runs produced **199** classification
records with 199 distinct source paths and zero duplicates — all 188 prose documents plus 11 task
lists that earlier manifests included. The remaining **64** task lists are covered deterministically
by `intel/task-completion-state.md`, which counts literal GFM checkboxes rather than relying on a
classifier. 188 + 75 = 263 and 188 + 11 = 199. **Every document is covered by one route or the
other.**

Final totals: **263 documents · 75 PRD / 124 DOC / 0 ADR / 0 SPEC · 554 requirements · 103 context
topics · 0 constraints · 0 decisions · 0 locked decisions · 0 blockers · 69 competing variants ·
112 informational entries · 11 ADR candidates · 0 cross-ref cycles in any run.**

**There is no run 6.** This document, `REQUIREMENTS.md` and `ROADMAP.md` remain structured so that a
future ingest from any source **appends** (a new milestone section, continuous phase numbering from
Phase 17) rather than restructures. Note: run-1 text in some files still says "run 1 of 14" — same
run, renumbered program.

**The Milestones 8-11 dependency graph is spent, and run 5 closed it out.**
`Milestones-8-11_Dependency-Graph.md` recorded M8 → M9 as a **hard** dependency ("M9 work should not
begin until M8 Epic 4 is complete"), M8 → M11 hard on path stability with M11 Epics 3-4 waiting on
M9 Epics 1-3, M9 → M11 hard on API stability, and M8 → M10 soft; critical path
M8 → M9 → M11 Epics 3-5 = 11-17 sprints, M10 entirely off it, release gates v0.2.0 / v0.3.0 /
v0.4.0 / v0.5.0. **Run 5 confirms every dependency was honoured and every gate was cut** — M9 100%
at v0.3.0, M10 100% at v0.4.0, M11 92% at v0.5.0, M12 99% at v0.6.0, which is exactly where the
tree sits. Keep its dependency semantics and release-gate criteria as a pattern; the schedule is
history.

**Constraint-shaped material is abundant and entirely untyped — in every run.** Run 3 was the most
constraint-dense set for build-system contracts, dependency layering and module boundaries; **run 5
is the most api-contract-dense**, being the first with a genuine HTTP surface carrying status
codes, headers, envelopes, security schemes and a machine-checked specification. Yet **0 SPEC-typed
documents exist anywhere**, so all of it lives as PRD acceptance criteria. `intel/constraints.md`
inventories what would bind if the carriers were re-tagged. The strongest candidates across the
corpus:

- the **`paladin-web` dependency-flow invariant** (run 5) — stated three times across two Epics
  with a mechanical verification command, `cargo tree -p paladin-web` must show no facade
  dependency;
- the **audit-suppression single-source invariant** (run 5) — the one constraint in the corpus the
  tree actively violates (SUPPLY-01);
- the **authentication / authorization / fail-closed / redaction contract** (run 5 M12 Epic 5 §4);
- the **crate dependency-direction invariant** (run 4) — currently violated once by
  `paladin-content → paladin-llm`;
- the **RustSec exception list with its expiry** (run 4);
- the **three dependency allowlists** and the **`config.yml` deserialization contract** (run 3) —
  both already contradicted by shipped code.

## Constraints

- **Tech stack**: Rust workspace (ten library crates + a `doc-examples` crate + the root
  `paladin-ai` facade), Tokio async throughout, Serde, SQLx, `thiserror` — pinned toolchain
  `rust-toolchain.toml` at 1.97.1. Not negotiable; the entire public surface is Rust traits.
  Shared dependency versions are pinned once in `[workspace.dependencies]` and referenced with
  `{ workspace = true }`.
- **Feature-gating is the compile-time contract**: `default = ["llm-openai"]`; per-provider
  `llm-openai` / `llm-anthropic` / `llm-deepseek` / `llm-all`; subsystem flags
  `content-processing`, `web-server` (axum), `notifications`, `vision`; storage flags `qdrant`
  (qdrant-client 1.14), `redis-queue`, `s3-storage`, `storage-mysql`, `openai-embeddings`; test
  flags `integration-tests`, `live-api-tests`; a `cli` flag that must never reach `default`; and a
  `full` convenience flag. `LlmPort` always compiles — only concrete adapters are gated.
  Unavailable adapters must fail at **compile time**, never at runtime, and `#[allow(dead_code)]`
  must not be used to paper over a `cfg` gate. Arsenal and its MCP transports are deliberately
  **not** feature-gated.
- **Dependency allowlists per crate are the enforcement mechanism for hexagonal purity** — and
  they are currently stale: `paladin-core` declares an "exhaustive" six and ships fourteen;
  `paladin-ports` declares seven and ships ten. The substantive invariant still holds (no LLM SDK,
  database driver, HTTP framework or object-storage client below the adapter layer). Reconciling
  the text with the tree is ARCH-03(b).
- **Architecture**: Hexagonal, dependencies flow inward only (core → nothing; ports → core;
  adapters → core + ports). Bypassing a port to import an adapter directly is an anti-pattern
  the codebase map calls out by name. The CLI is an **input adapter in the application layer**
  (`src/application/cli/`), not infrastructure — see the ADR-candidate note in Context.
- **Ubiquitous language**: Medieval military terms (Paladin, Battalion, Formation, Phalanx,
  Campaign, Chain of Command, Conclave, Council, Grove, Maneuver, Commander, Garrison, Arsenal,
  Armament, Citadel, Herald, Armory, Sanctum, Sentinel, Quest) are mandatory in code, docs and
  comments — they are the domain vocabulary, not decoration.
- **Error handling**: No `unwrap()`/`expect()`/`panic!` in library code; return `Result`. Layer-
  specific error enums converted at boundaries via `From`. `codebase/CONCERNS.md` lists existing
  violations to work down, not to imitate. Note the deliberate exception now shipping:
  `require_api_key()` in the live-API test harness panics by design — contested, see VERIFY-06.
- **Optional features degrade gracefully, never fatally**: RAG retrieval failure returns empty
  context and continues; memory extraction failure must not affect the Paladin response; a
  disabled autonomous layer must never fail core execution; Herald formatting errors fall back.
- **Methodology**: TDD (Red-Green-Refactor), rustdoc with compiling examples on all public
  items, `make clean-code` before committing, conventional commits.
- **Testing must work offline**: unit tests run with no external dependencies; anything needing
  Redis/MinIO/Qdrant/live APIs is feature-gated or `#[ignore]`d. The shipped three-tier strategy is
  Tier 1 always-in-CI, Tier 2 Docker-gated, Tier 3 API-key-gated. Provider API keys come from
  environment variables only — never CLI args, never config files, never logs.
- **Deploy targets**: Docker (distroless/slim, multi-arch amd64 + arm64) and Kubernetes.
  Image budget < 500 MB, pod startup < 30 s — both currently met.
- **Licensing/repo**: `github.com/DF3NDR/paladin-dev-env`. **The licence has three recorded
  answers** — the shipped root `Cargo.toml` says `license = "MIT"`, the M7 Epic 4 PRD and overview
  say MIT, and a signed decision checklist (approver `DF3NDR`, 2026-05-28, 551-package inventory)
  says **`MIT OR Apache-2.0`** with MPL-2.0 explicitly accepted for unmodified use. `deny.toml`'s
  permissive-only allow-list plus eight per-crate MPL-2.0 exceptions already follows the checklist.
  SEC-02 settles it; do not infer.
- **Dependency advisories are a gated, governed surface — and the governance gap is owner/expiry
  coverage, not synchronisation.** Both `cargo audit` and `cargo deny check` gate CI.
  **Corrected by ingest run 5:** `.cargo/audit.toml` and `deny.toml` **are in sync** on all five
  vulnerability advisories (`RUSTSEC-2023-0071`, `-2025-0111`, `-2026-0187`, `-2026-0194`,
  `-2026-0195`); run 4's "out of sync" finding is withdrawn. `deny.toml`'s ten extra entries are
  *unmaintained* notices — a different advisory class, labelled as such and authorised by Milestone
  10 Epic 4 FR-1 step 5. What remains: **13 of the 15 suppressions carry documented reasoning but
  no named owner and no expiry** (only the original two have a formal risk acceptance, owner
  Platform Security, **expiry 2026-09-30**); **Milestone 10 Epic 2 FR-3's schema requires neither**,
  so the configs comply and the policy is the gap; **the three 2026 vulnerability ignores are
  authorised by no ingested document**; and **`ci.yml` still runs two differently-configured
  `cargo audit` jobs** with the same display name, one bare and one with two inline `--ignore`
  flags. SEC-01 owns the set and the expiry; SUPPLY-01 deletes the duplicate job; SUPPLY-02 carries
  the owner/expiry schema and the three unratified ignores.
- **The `paladin-web` dependency-flow invariant is the strongest architectural rule in the
  workspace.** `paladin-web` depends on `paladin-ports` and `paladin-core` and **must not** depend
  on the `paladin-ai` facade; the registry and handlers depend only on the `PaladinExecutorPort`
  trait and the `Paladin` entity, and the concrete `PaladinExecutionService` and `AuthPort`
  implementation are injected at composition time by the server binary. Verified mechanically:
  `cargo tree -p paladin-web` must show no facade dependency. Stated three times across two
  Milestone 12 Epics — the clearest SPEC candidate in the corpus.
- **The HTTP agent surface is deliberately narrower than the embedded-library surface.** Agents
  served over HTTP are **LLM plus prompt only** — no Garrison memory, no Arsenal tools — stated
  once in a Milestone 12 Epic 2 non-goal and restated once by Epic 3. Whether that is planned scope
  or a permanent property of the topology is undecided (ORCH-04b), and the deployment-topologies
  decision matrix must say which.
- **Milestone 12's operational non-goals are load-bearing**: config is read once at startup (no
  hot-reload); the server binds **plain HTTP** and TLS is terminated by a proxy or ingress;
  authorization stops at `allowed_roles` plus an admin gate, with no finer-grained scopes; and
  configuration is **not** encrypted at rest — "secrets management is the operator's
  responsibility, as with LLM keys". API-key values should come from env or secret indirection, not
  committed config.
- **Crate dependency direction between leaf crates is contested, not settled.** M7 Epic 1 §6.1
  states "No extracted crate may depend on another extracted crate or on the `paladin` facade"
  absolutely; `crates/paladin-content` declares an optional `paladin-llm` edge behind its `llm`
  feature. Until HARD-05 restates the rule, treat leaf-to-leaf edges as requiring a decision rather
  than as permitted or forbidden.
- **`actix-web` is banned**, not merely unused: `deny.toml:99-103` denies it with the reason
  "paladin-web standardizes on axum; no second web framework". Reintroducing a second HTTP framework
  is a deliberate, reviewed decision — the guardrail is live and enforced in CI.
- **Edition is mixed, and the documents disagree too**: verified 2026-07-30, the root package
  and every crate declare `edition = "2024"` **except** `crates/paladin-ports` and
  `crates/paladin-notifications`, which declare `"2021"`. Milestone 5 Epics 1-4 require 2021 and
  Epic 5 plus the milestone overview require 2024, so neither the code nor the record is
  self-consistent. Builds succeed today but the mix is brittle (`codebase/CONCERNS.md`). ARCH-03(a)
  records the answer; REL-02 applies it.

## Key Decisions

<!-- LOCKED DECISIONS. See .planning/decisions/ for the full ADR text behind each row — this
     table links to it rather than restating it. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| [BattalionConfig field set](.planning/decisions/0001-battalion-config.md) (ADR-0001) | `battalion/mod.rs:37` (`REQ-battalion-config-v1`, Epic 4 FR-4.1) is the authoritative field set, confirmed by direct grep; the `citadel.rs:280` struct is a self-described placeholder, not a competitor, and is renamed `BattalionCheckpointConfig` with its serde shape unchanged rather than merged or deleted. | ✓ Good — applied in Phase 2 (`citadel.rs:284` now `BattalionCheckpointConfig`, serde shape unchanged) |
| [BattalionResult field set](.planning/decisions/0002-battalion-result.md) (ADR-0002) | The shipped struct at `battalion/mod.rs:549` is a merged superset satisfying all three competing positions (Epic 4 FR-4.2, Epic 5 FR-5, Epic 8 FR-7) at once, per `intel/code-verification.md`'s explicit "do not plan a reconciliation task" — a recording task, not a reconciliation. | conforms |
| [Formation minimum Paladin count](.planning/decisions/0003-formation-min-paladins.md) (ADR-0003) | Formation relaxes its minimum to one Paladin (an integer bound, 0 still rejected), resolving the live contradiction between `Formation::validate`'s ≥2 rejection and the Commander's passing `test_auto_selects_formation_for_single_paladin`; Majority aggregation's independent 3-Paladin minimum is untouched. | ✓ Good — applied in Phase 2 (`formation.rs:108-112` rejects only the empty case) |
| [Temperature validation](.planning/decisions/0004-temperature-validation.md) (ADR-0004) | Validation becomes provider-aware via a new `temperature_range: Option<(f32, f32)>` on `ProviderCapabilities`, falling back to the existing global `[0.0, 1.0]` when a provider declares none — making DeepSeek's documented `0.0–2.0` range reachable through the normal Paladin path for the first time. | ✓ Good — applied in Phase 2 (`llm_port.rs:777`; all three adapters populate it, DeepSeek 0.0-2.0 now reachable) |
| [Herald trait signature](.planning/decisions/0005-herald-trait.md) (ADR-0005) | The shipped trait at `herald.rs:49` ships the v2 fallible form (`Result<String, HeraldError>` throughout except the deliberately infallible `format_error`), matching Epic 8 §6.2 and making FR-10's graceful-degradation requirement expressible. | conforms |
| [Project-wide test coverage gate](.planning/decisions/0006-coverage-gate.md) (ADR-0006) | Measured **84.79%** workspace line coverage against commit `9be788c8e9c744ec3a6aad20b64110fb85925de4`, truncated to an **84%** hard-fail floor effective from the first run — freshly measured, not carried forward from a stale baseline. | — Pending — floor honoured by measurement (85.56% / 85.92% in Phase 3) but not yet wired into CI; PIPE-02, Phase 15 |
| [Workspace version is 0.7.0](.planning/decisions/0008-workspace-version-0-7-0.md) (ADR-0008) | The Milestone 6 facade change was breaking but already shipped inside the pre-1.0 series, so SemVer's pre-1.0 convention expresses it as a minor bump; `0.7.0` is the branch's own declared intent and the next lockstep figure in the ORCH-05 chain, confirmed by the human user 2026-08-03. | ✓ Good — executed in Phase 4; all twelve manifests and internal pins converged on 0.7.0 |
| [Workspace Rust edition is 2024](.planning/decisions/0009-workspace-rust-edition-2024.md) (ADR-0009) | Ten of twelve manifests already declared `edition = "2024"`; the toolchain (pinned `1.97.1`) has supported it since Rust 1.85, so the two stragglers (`paladin-ports`, `paladin-notifications`) were bumped forward rather than the other ten moved back. Closes `CONCERNS.md`'s false "does not exist in stable" claim. | ✓ Good — executed in Phase 4; `paladin-ports` and `paladin-notifications` bumped, 12/12 on edition 2024 |
| [Grove routing model from configuration](.planning/decisions/0013-grove-routing-model.md) (ADR-0013) | `GroveConfig.routing_model: Option<String>` replaces the hardcoded `"gpt-4"` literal; when absent under `RoutingStrategy::LlmRouting`, routing hard-errors with `BattalionError::RoutingError` and no fallback of any kind, a deliberate one-way runtime break the human user approved as `proceed-as-locked`, observable from `GroveExecutionService::execute()` — not only from internal routing helpers. | ✓ Good — field, builder setter and guard shipped by Phase 6 plan 06-01; `06-VERIFICATION.md` found the no-fallback guard unreachable from the public entry point (`route_task`'s blanket fallback arm intercepted it); closed by plan 06-08's pre-dispatch resolution in `route_task`, with `test_grove_llm_routing_errors_when_routing_model_absent_through_execute` as the named `execute()`-level exerciser |
| [Milestone 4-6 tier numbering](.planning/decisions/0014-milestone-4-6-tier-numbering.md) (ADR-0014) | The directory/task-list numbering is authoritative (Milestone 4 = Tier 1, 5 = Tier 2, 6 = Tier 3); every internal "Milestone 1/2/3" reference in these three milestones' own documents is a tier label, not a competing milestone number, closing the second of the corpus's two numbering collisions alongside ADR-0010. | conforms |
| [`paladin-core` / `paladin-ports` dependency allowlist](.planning/decisions/0015-core-ports-dependency-allowlist.md) (ADR-0015) | Rebaselines the "six-crate, exhaustive" allowlist against the measured fourteen/eleven dependencies and states the enforceable invariant (no provider SDK, transport client, storage driver or web framework) independently of the count; `tokio` in `paladin-core` gets its own written justification. | conforms |
| [Port value-type ownership](.planning/decisions/0016-port-value-type-ownership.md) (ADR-0016) | `paladin-core` owns `PaladinResult`, `StopReason` and `TokenUsage`; `paladin-ports` re-exports them. Promotes the Epic 1 Approved-but-DOC decision record into the ADR corpus so it outranks the later PRD that would have reintroduced the upward dependency it removed. | must change — Phase 8 / DEBT-05 collapses the two duplicate `TokenUsage` copies into re-exports of the canonical definition |
| [LLM configuration bridge location](.planning/decisions/0017-llm-config-bridge-location.md) (ADR-0017) | The shipped `crates/paladin-llm/src/config/bridge.rs` location is accepted; Epic 4's circular-dependency concern was real when written but the cycle no longer exists because Milestone 6 moved the config types down into `paladin-llm`, not the bridge up. | conforms |
| [Milestone 6 facade re-export policy](.planning/decisions/0018-m6-facade-reexport-policy.md) (ADR-0018) | The no-shim posture stands as policy; Milestone 6 removed publicly reachable import paths, a breaking change absorbed as a minor bump under pre-1.0 semantics per ADR-0008. Clause (iv) amended 2026-08-06 (plan 07-13): the pre-existing battalion shim mechanism outlived the retired `use_cases/` directory under a Milestone 8 rename. | conforms |
| [Binary-target architecture](.planning/decisions/0019-binary-target-architecture.md) (ADR-0019) | Ratifies the three shipped binary targets (`paladin`, `paladin-cli`, `paladin-server`), each with a stated purpose, and records that `structopt`'s only consumer is the un-gated `paladin` binary and that `paladin-herald` re-introduces two of the three CLI-only dependencies unconditionally. | must change — Phase 16 writes the mdbook page; Phase 8's CLI-isolation requirement inherits the `structopt`/`paladin-herald` re-scoping |
| [Build-time benchmark target restated per scenario](.planning/decisions/0020-build-benchmark-per-scenario.md) (ADR-0020) | Transcribes the report's five per-scenario figures (two pass, three fail against ≥ 50%) and declines the report's own recommended re-measurement against a mid-tree baseline, with the reason recorded so no phase inherits an unfundable task. | conforms |
| [CLI placement in the application layer](.planning/decisions/0021-cli-application-layer-placement.md) (ADR-0021) | Ratifies `src/application/cli` as the CLI input adapter's home, closing `PROMOTION.md`'s second Phase-7-owned candidate; corrects an earlier research claim that the module declaration was un-gated — it is `#[cfg(feature = "cli")]`-gated at both the module and its re-export. | conforms |
| [Milestone 4 Epic 2 FR-8 deprecation requirement withdrawn](.planning/decisions/0022-deprecation-requirement-withdrawal.md) (ADR-0022) | `grep -rn '#\[deprecated' src crates` returns 0; the epic's own `DEPRECATIONS.md:81` IMMEDIATE DEPRECATION section names no candidate ("None identified yet..."). Zero `#[deprecated]` attributes is the correct terminal state for the 0.7.0 tree, not an unfinished task — manufacturing a deprecation to satisfy a grep would be dishonest closure. | must change — Phase 8 itself is the executor; plan 08-06 performs the three-way reconciliation (ADR-0022, `DEPRECATIONS.md`, `stable-api.md`) |
| [CLI dependency isolation and the binary/Herald surface](.planning/decisions/0023-cli-dependency-isolation.md) (ADR-0023) | ROADMAP criterion 4 is stricter than the `superseded by shipped code` verdict Phase 7 recorded — `structopt`, `colored` and `comfy-table` still compiled unconditionally into a library-only build. Migrates `src/main.rs` to `clap` v4 and gates the `paladin` binary behind `required-features = ["cli"]`; gives `paladin-herald` its first `[features]` section. | must change — Phase 8 itself is the executor; plans 08-07/08-08 perform both sites' code and the downstream build-surface sweep |
| [RustSec exception governance](.planning/decisions/0024-rustsec-exception-governance.md) (ADR-0024) | Four surfaces encoded this workspace's RustSec suppression posture at four different times by four different mechanisms; `SECURITY-EXCEPTIONS.md` becomes the one authoritative governance register (ten live rows, all eleven fields non-empty), extending M10 Epic 2 FR-3's four-field schema with owner and review date, ratifying the three unauthorised 2026 vulnerability ignores on concrete compensating controls, reassigning owner to `DF3NDR`, and renewing the 2026-09-30 acceptance to per-advisory `2026-12-31` dates. | must change — Phase 9 itself is the executor; plan 09-06 reconciles `deny.toml`/`.cargo/audit.toml` to the register and lands `scripts/check-advisory-register.sh` — SEC-01 |
| [Licence posture](.planning/decisions/0025-licence-posture.md) (ADR-0025) | Three positions were live (a signed `MIT OR Apache-2.0` checklist, an `MIT`-only PRD, and a shipped `license = "MIT"` manifest); the repository owner selected the dual expression via a blocking checkpoint. All eleven manifests now declare `MIT OR Apache-2.0`, with `LICENSE-MIT`/`LICENSE-APACHE` added and the PRD's single-licence claim annotated superseded. | must change — Phase 9 itself is the executor; plan 09-05 applies the expression across every manifest and licence file — SEC-02 |
| [crates.io name-collision guard](.planning/decisions/0026-crate-name-collision-guard.md) (ADR-0026) | Package-name collisions cost Epic 4 two renames and a NO-GO cycle, with no guard earlier than a release-time dry run. An offline, bidirectional allow-list guard (`.crate-names.txt` + `scripts/check-crate-names.sh`) now runs on every pull request; the accepted residual cost is stated explicitly — a genuinely novel name is still a human check against crates.io, not a CI one. | must change — Phase 9 itself is the executor; plan 09-04 lands the guard in the required `cargo-deny` CI job — SEC-03 |
| [`Dockerfile.chef` planner-stage supersession](.planning/decisions/0027-dockerfile-chef-planner-stage.md) (ADR-0027) | M7 Epic 2 FR-01's nine-manifest planner-stage enumeration was itself the staleness defect SEC-05 names, not just the one crate it omitted. Deleted rather than extended — planner-stage crate coverage is now structural (`COPY crates ./crates`), citing cargo-chef's own upstream documentation for why the enumeration never delivered the cache-tightness claimed; the caching claim itself is recorded as not measured (Docker is absent from this environment). | must change — Phase 9 itself is the executor; plan 09-03 performs the deletion — SEC-05 |
| [Milestone 8's authoritative account — the 2026-06-04 reconciliation](.planning/decisions/0028-m8-reconciliation-authoritative.md) (ADR-0028) | Two Milestone 8 documents (`facade-audit.md`, `infrastructure-adapter-disposition.md`) classified ~4,400 LOC of orphaned, uncompiled duplicate files as "active bridges that stay"; the 2026-06-04 reconciliation is authoritative instead, with its orphan test preserved as a runnable procedure and three in-execution corrections (`paladin_registry.rs` consolidated not deleted, `sqlite_*_repository.rs` made non-optional not deleted, the rest genuinely orphaned) carried into the record. | conforms — Phase 10 plan 10-02 — HARD-02 |
| [Version trajectory — `v0.1.0-rc.1` as closed history](.planning/decisions/0029-version-trajectory-history.md) (ADR-0029) | `v0.1.0-rc.1` (commit `a9530fc`, 2026-05-28, all ten crates, GO sign-off) is recorded as closed history, not current state — the tree is four minors past it at `0.7.0`; REL-01 already converged on `0.7.0` (ADR-0008) and is not re-opened. A `## Trajectory` table is left for Phase 13 / ORCH-05 to append `v0.3.0`-`v0.6.0` to. | conforms — Phase 10 plan 10-03 — HARD-03 |
| [Milestone 7's self-numbering collision — directory numbering is authoritative](.planning/decisions/0030-milestone-7-self-numbering.md) (ADR-0030) | The Milestone 7 overview titles itself "Milestone 4" and credits "Milestones 1-3" with work the directory numbering assigns to Milestones 4-6 — the fourth instance of this corpus's numbering collision, citing ADR-0010 and ADR-0014 as precedent and discharging the Roadmap Extension Protocol's predicted fifth instance. | conforms — Phase 10 plan 10-03 — HARD-04 |
| [The extracted-crate dependency rule as a default-build invariant](.planning/decisions/0031-extracted-crate-dependency-rule.md) (ADR-0031) | M7 Epic 1 PRD §6.1's absolute "no extracted crate may depend on another extracted crate or the facade" is restated as a default-build invariant — checkable via `cargo tree --no-default-features` — legalising `paladin-content`'s existing non-default, facade-gated `llm` feature rather than removing a shipped capability. | conforms — Phase 10 plan 10-04 — HARD-05 |
| [PDF extraction is unconditional; the inert `pdf` feature and the -0187 reachability path](.planning/decisions/0032-pdf-extraction-capability.md) (ADR-0032) | `pdf = []` gates no dependency and no code while `pdf-extract` is an unconditional dependency of `paladin-content` — PDF extraction ships in every build regardless of the feature. The inert feature is deleted (not wired), the consumer-visible cost recorded in `CHANGELOG.md`, and `.cargo/audit.toml`'s `RUSTSEC-2026-0187` reasoning corrected to the true reachability path. | must change — Phase 10 itself is the executor; plan 10-05 performs the deletion and the two config corrections — HARD-06 |
| [One `cargo doc` bar — ratified, measured, and its residue](.planning/decisions/0033-cargo-doc-warning-bar.md) (ADR-0033) | `cargo doc --workspace --no-deps` already ships a zero-warning gate in the required `lint` CI job — ratified as the project's one bar over M8 Epic 5 FR-19's weaker "warnings acceptable". The tree's measured red state (exit 1, 20 warnings across `paladin-web`/`paladin-battalion`/`paladin-ai`/`paladin-herald`) is recorded with Phase 16 / DOCS-03 as owner; DEBT-03 recorded discharged by Phase 8. | must change — Phase 10 itself is the executor; plan 10-06 brings `release-check`'s doc-test step to CI's strength; the warning residue itself is Phase 16 / DOCS-03's — HARD-07 |
| [D1–D4 facade relocation disposition](.planning/decisions/0034-d1-d4-facade-relocation-disposition.md) (ADR-0034) | Each of D1 through D4 carries a verb, a named owner, and — where deferred — a concrete trigger, instead of an effort/risk rating: D1 defers to a facade-wide no-alias sweep, D2's `user_service.rs` split is withdrawn to a three-owner split, `content_service.rs`/`event_manager.rs` and D3/D4 each defer to their own named trigger, and no relocation executes this phase. | conforms |
| [`paladin-ml` leaf-crate placement condition](.planning/decisions/0035-paladin-ml-leaf-crate-placement.md) (ADR-0035) | Any future TensorFlow adapter goes into a dedicated `paladin-ml` leaf crate with the `ml` flag on that crate, never back into the facade, and `paladin_ports::input::ml_port::MlPort` stays in the workspace — promoting the condition out of a DOC's precedence into an ADR without creating the crate or reintroducing the adapter. | conforms |
| [The audit-suppression single-source topology invariant](.planning/decisions/0036-audit-suppression-single-source-topology.md) (ADR-0036) | An advisory suppression may be expressed only in `.cargo/audit.toml` or `deny.toml`, never as an inline `--ignore` flag in a `.github/workflows/*.yml` file; ADR-0024 owns suppression *contents* (which advisories, whose sign-off), this ADR owns suppression *topology* (which files may legally carry one at all), and `scripts/check-workflow-suppressions.sh` — wired into `make check-gates` and `ci.yml`'s `cargo-deny` job — is what turns the invariant into a gate rather than a comment. | conforms |
| [The agent route surface is `/v1`](.planning/decisions/0037-agent-route-surface-v1.md) (ADR-0037) | Four Milestone 12 Epics' unprefixed route text (`POST /agents/{id}/execute`, etc.) is superseded provenance, not a live contract; the committed `crates/paladin-web/openapi.json` drift-guard baseline settles the question by construction — all six agent paths are `/v1`-prefixed, live-tested by `spec_paths_are_versioned_under_v1` and drift-guarded by `openapi_matches_committed_baseline`. The one live consequence, `docs/src/deployment-topologies/sidecar.md:29`'s unprefixed route reference, is corrected to match. | must change — Phase 13 itself is the executor; plan 13-08 performs the `sidecar.md` correction — ORCH-03(a) |
| [`AgentProvisioner` placement — stays in `paladin-web`](.planning/decisions/0038-agent-provisioner-placement.md) (ADR-0038) | `AgentSpec`, the type the trait's only method takes, derives `utoipa::ToSchema` and is documented as sent in the body of `POST /agents` — an OpenAPI-annotated HTTP request DTO, not a portable core type; `paladin-ports` carries no `utoipa` dependency, and promoting the trait there would be the first `paladin-ports` dependency whose entire reason to exist is web-framework documentation tooling, exactly the class ADR-0015 Decision (i) bars. Ratified at plan 13-09's blocking checkpoint by a human operator (D-00i). | conforms — Phase 13 plan 13-09 — ORCH-04(a) |
| [Garrison and Arsenal absent from HTTP-served agents — a permanent topology property](.planning/decisions/0039-http-topology-no-garrison-no-arsenal.md) (ADR-0039) | The absence of Garrison (memory) and Arsenal (tools/MCP) wiring on HTTP-served agents, previously stated once in a Milestone 12 non-goal, is ratified as a **permanent property of the shipped topology** rather than planned scope — `AgentSpec` has no fields for memory or tool configuration, and expressing an MCP server's identity, credentials and lifetime in a JSON request body is genuine API design no milestone has scheduled. `docs/src/deployment-topologies/http-service-host.md` and `overview.md` now state the limitation in prose. Ratified at plan 13-09's blocking checkpoint by a human operator (D-00i). | must change — Phase 13 itself is the executor; plan 13-09 performs both doc-page corrections — ORCH-04(b) |

**v0.9.0 (Phases 18-21) minted no new ADRs.** Its decisions were recorded as per-phase locked
decisions (`D-xx`) in each phase's `CONTEXT.md`/`DISCUSSION-LOG.md`, now archived under
`.planning/milestones/v0.9.0-phases/`. The four with standing consequences, all applied in the
tree and none contradicted since: the **CodeQL disqualified-advisory posture** (verdict
version-scoped to CodeQL `2.26.3`; the probe fixture and a written re-probe trigger stay in the
tree; `codeql.yml` deliberately not pinned in any ruleset — ✓ Good); **Trusted-Publishing ratchet
order** (prove the OIDC path with a real publish before revoking the standing token — ✓ Good,
exercised live); **registry state over error prose** for publish idempotency (crates.io index as
the source of already-published truth, bounded polling instead of `sleep 20` — ✓ Good, proven by
the rc.3/rc.4 recovery rehearsals); and the **curated-changelog release body with no git-log
fallback** (a missing section fails the release — ✓ Good, proven on `v0.8.1-rc.5`). The
signing/provenance question was examined and **deferred with recorded reasoning**
(`docs/src/appendix/release-automation.md`, naming `actions/attest-build-provenance` as the
future candidate) — Pending.

Six competing-variant pairs are Phase 1's scope (`BattalionConfig`, `BattalionResult`, Formation
minimum Paladin count, temperature validation, the Herald trait signature, and the coverage gate);
all six are recorded above. See `.planning/decisions/` for conventions, the numbering index and the
full ADR text — this table links to each ADR rather than restating it.

**Before this phase, this table was empty by evidence, not by omission — and that emptiness was
itself the corpus's most notable structural finding.** Twelve milestones, eighteen months and 554
requirements produced **not one protected decision** anywhere in the ingested corpus. Every
technical position in the entire history of this project — the workspace decomposition, the port
ownership rules, the feature-flag contract, the auth mechanism, the advisory suppressions, the
licence posture — sat at PRD or DOC precedence and was auto-overridable by the next document that
mentioned it. Three consequences, all observed rather than hypothesised:

- **No LOCKED-vs-LOCKED contradiction was ever possible.** That is why 69 competing variants
  produced **0 blockers** across five runs: there was never a pair of protected positions that
  could collide.
- **Mechanical precedence has already produced at least one architecturally wrong answer** — a PRD
  outranking an Approved-status decision record, with a rule that would reintroduce the exact
  upward dependency that decision removed (ARCH-03(c), variant group 19).
- **The shipped tree is the arbiter by necessity, not preference.** Precedence runs
  **ADR → shipped tree → codebase map → `intel/code-verification.md` → PRD → DOC → checkbox**.

Everything asserted in the ingested PRDs and DOCs is **supersedable** — demonstrated, not
theoretical: run 2 produced eight documented supersessions of run-1 requirements, run 3 produced
eleven more including the entire monolith → workspace path migration, run 4 produced eleven more
still including the corpus's first document-supersedes-document notice, and run 5 produced twelve
more including the first case of **a later run correcting an earlier run's direct code
verification** (see *Superseded but preserved* in `REQUIREMENTS.md`). The first real entries in
this table are expected from Phase 1 (six ADRs, one per competing variant pair), Phase 5 (four
recorded answers), Phase 7 (six more), Phases 9-10 (the RustSec exception set, the licence posture,
the leaf-crate dependency rule, the PDF capability and the `cargo doc` bar), Phase 12 (the advisory
governance schema and the ADR-promotion decision), Phase 13 (the two Milestone 12 seams) and
Phase 14 (the token mechanism).

**Phase 4 entered two protected decisions above** (ADR-0008, the workspace version; ADR-0009, the
workspace Rust edition), ahead of the phases this paragraph originally forecast as their owners.
REL-01/REL-02's own convention — "whichever of Phase 4 / Phase 7 executes first records the answer,
the other applies it" — is why: Phase 4 runs before Phase 7 in the roadmap, so it recorded both
answers itself rather than leaving them at DOC precedence for nine more phases. Phase 7's ARCH-04
and ARCH-03(a) inherit these two rows instead of re-deciding them.

**Amended by Phase 9 (plan 09-07), dated 2026-08-08:** Phase 9 supplied its four forecast entries —
ADR-0024 (the RustSec exception set), ADR-0025 (the licence posture), ADR-0026 (the crates.io
name-collision guard) and ADR-0027 (the `Dockerfile.chef` planner-stage supersession) — all four
above. Of the two-Phase forecast "Phases 9-10 (the RustSec exception set, the licence posture, the
leaf-crate dependency rule, the PDF capability and the `cargo doc` bar)", Phase 9 closed the first
two; the leaf-crate dependency rule, the PDF capability and the `cargo doc` bar remain Phase 10's to
record (HARD-05, HARD-06, HARD-07). The original forecast paragraph above is retained verbatim,
per the amend-at-source convention.

**Eleven ADR candidates now exist, and none is entered here** — doing so would manufacture a locked
decision from a DOC-precedence assertion. The two with a live operational cost come first, and they
are the same subject from two different milestones:

1. **`Milestone_5/Epic_1/decisions/battalion-result-upward-dependency-decision.md`** (run 3) — the
   only decision/options pair in all 263 documents, carrying `Status: Approved`,
   `Decision Date: 2026-05-13`, `Chosen Option: Option A`, a Rationale, a Rejected Options section
   and an implementation checklist. It settles where `PaladinResult`, `StopReason`, `TokenUsage`,
   `RegistryError` and `HandoffError` live, and shipped code implements it. It is manifest-typed
   DOC, so a PRD published two days later outranks it — and that PRD's rule would undo the fix.
   **This is the strongest candidate in the corpus and the one with real consequences if left
   unprotected.**
2. **`Epic_17.5/epic17-5.md`** (run 2) — the CLI belongs in `src/application/cli` because "CLI is
   an input adapter in the application layer, not infrastructure". Also already applied in code,
   also outranked by a PRD that says otherwise.
3. **`Milestone_7/Epic_4/rustsec-remediation-plan.md`** (run 4) — a formal **risk acceptance**:
   two advisories, **owner Platform Security (Milestone 7)**, **review/expiry target 2026-09-30**,
   with compensating controls and required exit evidence. **The only item in all 263 documents
   carrying an expiry date, and the only candidate where not promoting it has an ongoing
   operational cost** — nothing else in `.planning/` will surface that date. Its governing epic
   states the acceptance criteria it satisfies. SEC-01 acts on the drift; promotion is a separate,
   user-owned step.
4. **`Milestone_7/Epic_1/cost-benefit-assessment.md`** (run 4) — a go/defer record with an explicit
   **"Self-Approval (Task 1.6)"** block, a named approver and an approval date of 2026-05-25,
   scoring four candidate extractions on four criteria with measured evidence, issuing four Go
   decisions and fixing an extraction order. Its governing PRD calls it "the authoritative source of
   record for *why* a decision was made". Everything an ADR needs except the type tag.
5. **`Milestone_7/Epic_4/license-compatibility-decision-checklist.md`** (run 4) — a licensing policy
   with a named approver (`DF3NDR`), an approval date (2026-05-28), a 551-package inventory and an
   explicit accept-or-replace decision on MPL-2.0. Contradicted by the shipped manifests → SEC-02.
6. **`Milestone_8/facade-cleanup-RECONCILIATION-2026-06-04.md`** (run 4) — an explicit supersession
   notice carrying `Supersedes (corrects):` two named documents, which then resolved all six of its
   own listed open decisions **in execution rather than by a recorded decision**. The same
   "resolved by outcome" pattern run 3 flagged for the binary-target question → HARD-02.
7. **`Milestone_10/Epic_2/prd-dependency-security-license-compliance.md` FR-1 + §8** (run 5) — the
   **audit-suppression single-source invariant**: exceptions live only in `audit.toml` and
   `deny.toml`, "so the workflow and the config cannot drift", with "no inline advisory-ignore
   flags remain in CI" as a success metric. **The tree violates it today** (SUPPLY-01). Paired with
   candidate 3, this is the promotion with the clearest payoff: together they would make the
   supply-chain posture a gate rather than an observation → SUPPLY-03.
8. **`Milestone_9/Epic_5/prd-user-admin-system-completion.md` §6.1** (run 5) — the
   opaque-bearer-token decision, with rationale (no `jsonwebtoken` dependency, no signing-key
   management, immediate revocation that stateless JWTs cannot do), a rejected alternative, and a
   recorded trade-off about multi-process deployment. **The only decision in the corpus that a
   later milestone contradicts in prose while silently preserving in code** → WEB-01, WEB-02.
9. **`Milestone_9/Epic_4/prd-agent-orchestrator-bridge.md` §6.1** (run 5) — **the cleanest
   ADR-shaped section anywhere in the corpus**: a four-criterion comparison table, a `(CHOSEN)`
   column header, an explicit decision, and the rejected option preserved as a future non-breaking
   enhancement. Everything an ADR needs except the type tag.
10. **`Milestone_12/Epic_1/prd-agent-registry-execution-api.md` §7 + OQ-2** (run 5) — the
    `AgentProvisioner` placement, recorded as a *default* ("keep in `paladin-web`; promote only if
    a second consumer appears") rather than a decision, while two shipped deployment-topology pages
    describe would-be second consumers → ORCH-04(a).
11. **`Deferred-QA-CICD-Completion/DEFERRED_COVERAGE.md`** (run 5) — the coverage deferral record,
    with a named sign-off ("AI Coding Agent (Epic 24 execution), February 14, 2026") and a "Next
    Review: Epic 27 or Epic 28 planning" trigger that was never reached. Weaker than the others: its
    two module paths are stale and its baselines predate Milestone 9 → DEFER-01 … DEFER-03.

Promoting any of these requires re-tagging the source document via `--manifest` and re-running
ingest. **Note that the ingest is closed** — promotion is therefore a deliberate, separate,
user-owned action rather than something a subsequent run will do incidentally.

**`.planning/decisions/PROMOTION.md` now carries the promotion procedure and an explicit owner
phase for all eleven candidates above** — Phase 1 built the mechanism (this file's numbering
scheme, required headings, and supersession rule, plus the worked example at
`.planning/decisions/0005-herald-trait.md`) but promotes none of the eleven itself.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):

1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):

1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-01 after **milestone v0.10.0 "Durable Agent Execution Runtime" started** —
scope taken from the approved design corpus in `.project/v0.10.0/` (program overview, seven epic
PRDs `ENG`/`CF`/`HITL`/`FT`/`RT`/`PLAT`/`OBS` carrying ~135 FRs, traceability matrix). 44 active
requirements minted across eight categories in a fresh `.planning/REQUIREMENTS.md`; new phases
start at Phase 22. Domain research skipped by recorded decision: the PRD corpus is itself the
design/research artifact, complete with test plans, acceptance criteria and a gap-analysis
traceability matrix. One PRD-vs-tree conflict recorded at scope time (RT-FR-20…22 provider
adapters already shipped in v0.8.0 — scoped as conformance close-out, not greenfield).*

---
*Last updated: 2026-09-01 after the **v0.9.0 "Security Tooling" milestone close** (Phases 18-21,
25 plans, 20/20 requirements, 240 commits `48ac11a5..3957d701`, 2026-08-24 → 2026-09-01). Audit
`tech_debt`, 0 blockers, 5 debt items with owners; every declared human-verification backstop
closed by recorded UAT before the close. Closeout type `override_closeout` on the strength of one
acknowledged open artifact (the user-owned coverage-reproduction todo), not any unverified phase.
Archived: `milestones/v0.9.0-ROADMAP.md`, `v0.9.0-REQUIREMENTS.md`, `v0.9.0-MILESTONE-AUDIT.md`,
`v0.9.0-phases/`. No git tag cut (main-only tag enforcement; closed on `chore/21-close`). Next:
`/gsd-new-milestone` — new phases start at Phase 22.*

---
*Last updated: 2026-09-01 after **Phase 21: Release Artifacts — Curated Release Notes and Attached
Distributables** completed and verified — the final phase of v0.9.0 Security Tooling. The release
body is now the curated `CHANGELOG.md` section (missing section fails, no git-log fallback); all
three declared binaries build under explicit features with existence asserts; the image is bound to
the release by immutable digest; checksums verify in one command; the archived-actions/`upload_url`
era is removed; and the whole path was proven live on throwaway tag `v0.8.1-rc.5` (run 33436573814,
all 12 jobs green) with both residual items closed by human UAT (out-of-band pull-by-digest;
`paladin-cli` executed from the released archive). ARTIFACT-01 … ARTIFACT-06 complete — v0.9.0 is
20/20 requirements. Signing/provenance examined and deferred with recorded reasoning
(`docs/src/appendix/release-automation.md`). Milestone close-out is the next step.*

---
*Last updated: 2026-08-24 after **Phase 16: Documentation Currency & the Architecture Gap**
completed and verified — 14 plans across 6 waves, all four requirements (DOCS-01 … DOCS-04) closed,
`16-VERIFICATION.md` `passed` on its second pass at 4/4 must-haves. This phase closed the **last
open checkbox count in the ingest corpus**. Its finding is worth recording because it reframes what
"documentation currency" meant here: the fourteen files were not stale, they were **fabricated**.
`logging.md` documented a `tracing` ecosystem with **zero `tracing` call sites** in the tree (the
real facade is `log` + `env_logger` behind a custom `LogOrchestrator`/`LogPort`); `monitoring.md`
and `troubleshooting.md` documented a `/metrics` endpoint with no dependency and no route;
`output-formatting.md` documented two built-in formatters (`HtmlHerald`, `CodeHerald`) that exist
nowhere; `memory-management.md` fabricated the `GarrisonConfig`/`SqliteGarrison` builder API, a DB
migration schema, and a `VectorGarrison` type (the real subsystem is Sanctum); `production.md`
fabricated an OAuth2/three-role RBAC section over the real two-role bearer/`x-api-key` mechanism;
and both `production.md` and `cicd.md` recommended **Snyk**, the scanner this project measured and
removed on 2026-08-18. Presence and mtime settled nothing — every one of the fourteen carries an
evidence-bearing verdict row in `16-DOCS-01-VERDICTS.md` citing the command or `file:line` that
produced it. [ADR-0047](.planning/decisions/0047-architecture-appendix-disposition.md) settles
DOCS-02: the 311-line architecture appendix is **archive material**, says so in a banner, and
points at the live chapter; Sentinel — the one component of 19 missing from that chapter — was
given a home there rather than rebuilding a second competing architecture document. DOCS-03 holds
one `cargo doc` bar at **zero warnings** workspace-wide with all **76** D-05 entry points carrying
`# Examples`, and the 30 new examples are genuinely compile-and-run: **no `no_run`/`ignore`/`text`
fence was added by any plan**, verified by classifying every fence in every diff. Two items are
recorded open rather than closed quietly: FR-26.3's "79 entry points" is **not reproducible** from
the tree (11 + 35 + 30 = 76; the delta is isolated entirely to the `*Service` count and attributed
to a stale figure, with derivation commands recorded), and the Charm APT signing key behind DOCS-04's
`vhs` install **could not be corroborated against any source independent of `repo.charm.sh`** — the
project owner authorised the install accepting that gap, and both devcontainer Dockerfiles were
corrected after they initially claimed a "human-verified out-of-band" check that never happened.*

*Last updated: 2026-08-23 after **Phase 17: Additional LLM Provider Adapters** completed and
verified — 22 plans (11 planned, 11 added across three `/gsd-plan-phase 17 --gaps` runs), all four
requirements (PROV-01 … PROV-04) closed, `17-VERIFICATION.md` `passed` on its **fourth** pass at
16/16 must-haves. This was the **first phase beyond the ingest-derived roadmap** — not ingest
material, user direction of 2026-08-15. [ADR-0045](.planning/decisions/0045-additional-llm-provider-selection.md)
records the selection study with its criteria written down before any candidate was scored:
**build** — Kimi, Qwen, Grok, Ollama, Gemini; **reject, already covered by the generic
operator-configured OpenAI-compatible provider** — Groq, Together, Mistral, Fireworks, Bedrock;
**Meta/Llama** dispositioned by naming Ollama as the host it actually targets, since "Llama" names
a model family and not a provider. Every verdict was human-selected in an interactive
`/gsd-discuss-phase` session, none `--auto`-derived (D-00i). Six adapters ship: five named presets
over a shared `CompatEngine`, plus one generic `base_url`-configured provider. The engine's
`CompatRequestParameters` mechanism gates the five optional sampling parameters **per preset with
no vendor-name branching** — Grok declares `frequency_penalty`/`presence_penalty` absent because
xAI rejects them by presence, Kimi declares `temperature`/`top_p` absent because Moonshot enforces
fixed values. Gemini is not built on `CompatEngine` at all; `generateContent` is its own shape.
Two further ADRs: 0046 (facade LLM feature-flag wiring), and **0004 amended in place** — the
temperature-range gate in `PaladinBuilder::validate()` now fires only when the caller actually
expressed a temperature (`manual_temperature_override`), not unconditionally, and the ADR's text
was brought back into agreement with the shipped code rather than left to teach a stale contract.
CI at `76b859d`: 46 success, 3 skipped, 0 failures, re-queried first-hand by the verifier (which
corrected the orchestrator's own "44 success" count). The `Coverage` job runs
`cargo llvm-cov --fail-under-lines 82`, so its success **is** the ADR-0006 floor holding with all
nine adapters counted — the job emits no percentage, so the record carries the verdict and not a
number, and the earlier 85.01% figure is superseded rather than re-cited. The live four-vendor
smoke (8/8 probes on shipped defaults, no overrides) is **relayed evidence**, not verifier-executed:
no vendor credentials or egress exist in that sandbox. Two `WINDOWS.md` rows stay open by design —
id 14 (a compose healthcheck never validated by `docker compose config`) and id 19
(`.project/current-exports.txt` generated under default features only, carried forward as accepted
debt IN-01 by a recorded human decision in an interactive checkpoint, not an `--auto` inference).
Rows 15-18 are waived: the Snyk mandate was **removed** on 2026-08-18 after measurement — Snyk Code
found 0 of 4 planted vulnerabilities in Rust that it caught in equivalent JavaScript, and Snyk Open
Source has no Cargo support, so a "clean" result there meant nothing was analysed. The known gap it
leaves — **no static taint analysis for first-party Rust** — is stated plainly in
`.github/instructions/security.instructions.md` rather than papered over, with manual
credential-handling review as the standing control. The PROV-* bullets remain in Active pending the
v0.7.2 milestone close, per this project's convention of graduating requirements to Validated at
ship time.*

*Last updated: 2026-08-10 after **Phase 13: Milestone 9-12 Ground Truth & Recorded Account**
completed and verified — 13 plans across 4 waves, all five requirements (ORCH-01 … ORCH-05) closed
with cited evidence, UAT 1/1 passed, security `threats_open: 0`. This was the **last ground-truth
phase in the corpus**: `.planning/ledgers/milestone-09-12.md` is the fifth and final as-shipped
ledger, carrying all 120 Milestone 9-12 requirements as cited verdicts, and `REQUIREMENTS.md`'s
120-row draft is reduced to a pointer at it. Three ADRs: 0037 (the agent route surface is `/v1`,
correcting the one shipped mdbook page that named an unserved path), plus 0038 (`AgentProvisioner`
stays in `paladin-web`) and 0039 (Garrison and Arsenal absent from HTTP-served agents is a
**permanent** topology property, not planned scope) — both ratified at plan 13-09's blocking human
checkpoint rather than auto-resolved, with provenance recorded per D-00i. ADR-0029's version
trajectory is completed through `v0.6.0` and `PROMOTION.md` advanced to `0040`. Zero `.rs` files
were touched across all 13 plans — the phase's D-19 prohibition held and is independently
re-measured in the ledger's close-out amendment. Four forward hand-offs carry the residue to Phases
14, 15 and 16. One accepted residual risk (`AR-13-01` / threat T-13-20): `sidecar.md` embeds
`crates/doc-examples/src/sidecar.rs:34` via mdBook `{{#include}}`, and that example still posts to
the unprefixed `/agents/{id}/execute`; the original mitigation greped the markdown source, which
never contains the included literal, so the check ran in the wrong layer. Phase 14 owns the fix.
The ORCH-* bullets remain in Active pending the v0.7.2 milestone close, per this project's
convention of graduating requirements to Validated at ship time.*

*Last updated: 2026-08-09 after **Phase 11: Facade Residue & Deferred Register Disposition**
completed and verified — 5 plans, all four requirements (FACADE-01 … FACADE-04) satisfied, UAT
3/3 passed, security `threats_open: 0`. The phase wrote records, not code: D5's 17 `println!`
occurrences are recorded as deliberate rustdoc-example stdout rather than converted (the register
`.planning/registers/facade-01-rustdoc-stdout-disposition.md` corrects ROADMAP criterion 1 at
source); ADR-0034 replaces D1–D4's effort ratings with verbs, owners and triggers, withdrawing the
`user_service.rs` split to a three-owner split; ADR-0035 promotes the `paladin-ml` leaf-crate
placement condition out of DOC precedence without creating the crate; and the 20-row Milestone 9
candidate triage resolves to 14 done / 6 not a candidate / 0 still open, finding `paladin-arsenal`
and `paladin-sanctum` to be artefacts of a mis-written table rather than future crates. The
FACADE-* bullets remain in Active pending the v0.7.2 milestone close, per this project's
convention of graduating requirements to Validated at ship time.*

*Last updated: 2026-08-05 after **Phase 5: Milestone 2-3 Ground Truth** completed and verified —
13 plans across 10 waves, all six VERIFY requirements validated. `.planning/ledgers/milestone-02-03.md`
now carries all 118 run-2 requirements as cited verdicts: 64 satisfied (3 with caveat), 25 present
but unproven, 21 superseded by shipped code, 5 deferred with reason, and **3 genuinely outstanding**
— the inherited record was substantially more wrong about what was missing than about what was
broken. 141 unique cited paths, zero unresolved. Four ADRs settle the contested definitions: 0010
(epic numbering, corrected at source in `RELEASE_NOTES_MILESTONE_3.md`), 0011 (vision surfaces
coexist; the "absent" encryption requirement verified present-but-unwired), 0012 (live-API
missing-key behaviour), and 0006 amended in place — still the only coverage ADR. All three
previously-unverified blocks now carry written verdicts, reducing Phase 6's CLOSE-02 to three named
clusters: Epic 14 cluster 8.0, Epic 24 clusters 1.0 and 8.0. Epic 22 needs no work. Zero code files
were touched — the phase's own prohibition held across all 13 plans. Prior updates below.*

*Last updated: 2026-08-04 after **milestone v0.7.2 "Milestone 2-3 close-out" started** — Phases 5-6
scoped, 9 requirements (VERIFY-01 … VERIFY-06, CLOSE-01 … CLOSE-03). Phase 9's 2026-09-30 RustSec
acceptance deliberately deferred to a later milestone with ~8 weeks of margin. Prior updates below.*

*Last updated: 2026-08-04 after the **v0.7.1 "Milestone 1 close-out" milestone** shipped —
Phases 1-4, 38 plans, 88 tasks, 25/25 requirements verified. Nine ADRs (0001-0009) now hold every
contested definition; coverage measured at 85.92% against an 84% floor; all twelve manifests
converged on version 0.7.0 / edition 2024. Closed as `override_closeout` (one verification
override: Phase 1's timestamp, see MILESTONES.md). Audit status `tech_debt` — no unsatisfied
requirements, ten deferred items carrying named owners. Prior updates below.*

*Last updated: 2026-08-01 after **Phase 2: Functional Gap Closure** completed and verified
(GAP-01 … GAP-07 all closed; 11/11 plans). Prior update below.*

*Last updated: 2026-07-30 after **ingest run 5 of 5 — FINAL. THE INGEST IS COMPLETE.**
(`.project/Milestone_9-Classic-Orchestrator-Completion` +
`.project/Milestone_10-CI-Hardening-Release-Automation` +
`.project/Milestone_11-Documentation-Overhaul-Publish` + `.project/Milestone_12-Web-API` +
`.project/Deferred-QA-CICD-Completion` + `.project/project-management`, 46 docs; **cumulative 263
documents covered — 199 classified plus 64 task lists measured deterministically — 554
requirements, 86 forward requirements across 16 phases, 60 variant entries across 30 groups,
69 warnings, 0 locked decisions, 0 blockers, 11 ADR candidates**)*

---
*Last updated: 2026-09-02 after Phase 22 completion (v0.10.0 milestone; next: Phase 22.1
engine-readiness-defect-and-msrv-follow-up).*
