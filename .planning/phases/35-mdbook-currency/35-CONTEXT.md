# Phase 35: mdBook Currency - Context

**Gathered:** 2026-09-17
**Status:** Ready for planning
**Mode:** `--auto` — every decision below is the recommended default, auto-selected without a user
prompt; the alternatives considered are in `35-DISCUSSION-LOG.md`. The maintainer can overrule any
D-nn by editing this file before `/gsd-plan-phase 35` runs.

<domain>
## Phase Boundary

Phase 35 delivers **the mdBook at v0.10.0 currency**: every one of the **60 `MB-nn` rows** in
`34-AUDIT.md` §5 is closed by a page edit or a new page — the one `missing` page (the Phase 22
superstep engine, MB-30) is written and linked from `docs/src/SUMMARY.md` at the nav position the
audit assigned, and the 59 `stale` pages are brought to the shipped tree (API shapes, vocabulary,
crate lists, feature flags, version pins, MSRV, CLI flags, CI job names). `mdbook build docs/`
with the linkcheck backend stays green under the exact `docs.yml` sequence, snippets meant to run
are compile-verified through `crates/doc-examples`, illustrative snippets are fenced `rust,ignore`,
the vocabulary exit greps are empty, and `CHANGELOG.md` `[0.10.0]` carries a Documentation entry.

**Not in this phase:** rustdoc warnings and intra-doc links (`RD-nn`, Phase 36); `examples/`
programs, `examples/README.md` and edits to *existing* `crates/doc-examples` modules (`EX-nn`,
Phase 36); the release re-seal, merge and tag (Phase 37); any public API change (none is needed —
docs and a `publish = false` example crate do not move the surface); the five entries in the
Phase 34 deferred register (`deferred-items.md`), including the PROJECT.md corrections; and any
re-audit of pages the audit settled `current`. No page is deleted and no nav entry is removed.

</domain>

<decisions>
## Implementation Decisions

### Inherited — locked by earlier phases, not re-litigated
- **D-00a:** The work list is `34-AUDIT.md` §5 — 60 IDs, ordered MB-30 first then
  `docs/src/SUMMARY.md` nav order (Phase 34 D-21). Phase 35 may re-batch the rows into its own
  plans and waves but must close every ID; none is dropped, merged away or absorbed into another
  ID's fix (Phase 34 D-03). Each row's Location, Size and Cites cells are the contract.
- **D-00b:** The audit rows stand as measured. Verified 2026-09-17 at discuss time:
  `git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- . ':!.planning'` is empty, so
  the Phase 34 D-23 re-run rule is not triggered at planning. If Phase 36 lands source or
  `doc-examples` changes on the branch before this phase's final evidence run, the executor
  re-runs `mdbook build docs/` and `scripts/check-doc-examples.sh` rather than trusting the
  Phase 34 baseline.
- **D-00c:** Shipped tree outranks any document (Phase 34 D-00g). Every correction moves the page
  toward the tree; a page describing something the binary or crates do not have loses the claim.
  Nothing under `src/` or `crates/` other than `crates/doc-examples` changes in this phase.
- **D-00d:** Vocabulary: Phase 30 D-01 (domain roles use the Medieval-military term, units and
  technical ports keep industry names); `grep -rniE '\bQuartermaster\b' docs/src` must be empty
  (Phase 30 D-14); a bare token total beside a `TokenUsage` split is governed by Phase 31 D-08 with
  the Phase 31 D-29 hit list, already re-checked page by page in `34-AUDIT.md` §2 (only
  `domain-model.md` failed, and for a Garrison field, not a bare total).
- **D-00e:** The gate is the exact `docs.yml` sequence (Phase 34 D-11): `mdbook-mermaid install
  docs/` → `mdbook build docs/` with `[output.linkcheck] warning-policy = "error"` →
  `scripts/check-doc-examples.sh` → `scripts/check-doc-config.sh`. Local pins match `docs.yml`
  (`mdbook 0.4.40`, `mdbook-mermaid 0.13.0`, `mdbook-linkcheck 0.7.7`, recorded in the audit's
  Measurement Header). After `mdbook-mermaid install docs/`, `git status --porcelain -- docs` must
  be clean; if not, restore with `git checkout -- docs/` and record it (Phase 34 D-22).
- **D-00f:** The Upgrading page and migration pointers already agree with `MIGRATION.md` §9.1
  (4/4) and §9.8 (7/7) — `34-AUDIT.md` D-09 subsection. That half of ROADMAP SC1 is satisfied by
  recorded evidence and is **not** edited; the only migration-family item is MB-16, a
  self-consistency fix on `migration-guide.md` (D-06 below).
- **D-00g:** Requirement prefix is `CURR-*` (Phase 34 D-20; the ROADMAP's "assigned at planning
  under the Phase 34 prefix"). `CURR-01…05` are spent; the planner mints the next numbers for
  Phase 35 in `.planning/REQUIREMENTS.md`.
- **D-00h:** Runnable documentation snippets live in `crates/doc-examples` behind
  `// ANCHOR:` regions and reach the page via `{{#include ../../../crates/doc-examples/src/<mod>.rs:<anchor>}}`
  (Phase 16; `scripts/check-doc-examples.sh` Layer 1). `docs/src/architecture/commissary.md`'s
  usage sketch keeps mirroring a real unit test (Phase 30 D-06, Phase 32 D-12).

### Page disposition — correct, archive, or retitle (the 25 appendix items and two nav mismatches)
- **D-01:** Every `MB-nn` page is closed under exactly one of three dispositions, chosen by the
  nature of the content and recorded in the closure table (D-23): **correct** — the page is live
  reference for a shipped surface; its content is brought to the tree; **archive** — the page is a
  dated snapshot, report or completion summary whose only value is historical; it gets the
  ADR-0047 banner (`docs/src/appendix/design-and-architecture.md` lines 3-9 are the template)
  naming the live source of truth, plus the one-line factual corrections its §5 finding cites,
  and nothing more; **retitle** — the nav title and the page content disagree; the nav entry is
  fixed to match the content and the missing subject gets its own small page. **No page is
  deleted and no entry leaves `docs/src/SUMMARY.md`** — archiving records a disposition, it does
  not destroy the record (ADR-0047, Phase 16 D-00d), and inbound links stay valid. — **Reversibility:**
  reversible — a banner or a nav retitle is a one-commit edit either way.
- **D-02:** Archive-tier membership, settled from the §2 findings: `appendix/doc-coverage-report.md`
  (MB-01 — the Phase 34 deferred note asked this phase to choose archive vs regenerate; archive,
  with the banner pointing at ADR-0033's zero-warning bar and Phase 36 as the live measure, because
  a regenerated report would be invalidated by Phase 36 within the same milestone);
  `appendix/build-baselines.md` (MB-39 — a dated Milestone 7 build-time snapshot; banner plus the
  "M7 Current (10-crate)" framing corrected to say it is a snapshot); `appendix/user-system.md`
  (MB-60) and `appendix/user-rest-api.md` (MB-59) — banners state plainly that the `paladin user`
  CLI they describe does not exist in the shipped `paladin-cli` binary while the user
  domain/service/repository layers do (`crates/paladin-storage/src/sqlite_user_repository.rs`,
  `crates/paladin-core/src/platform/manager/user_service.rs`), and that WEB-01/WEB-02 remain
  forward work; `user-rest-api.md`'s malformed structure (opens as a raw code block, ends
  mid-source) is repaired only enough to render as a page; `appendix/contributing-legacy.md`
  (MB-47 — banner pointing at `contributing/development-setup.md`, plus MSRV 1.88 and the
  `crates/` workspace line). Pages not named here or in D-03 are assigned by the planner under
  the D-01 rule, and the assignment is written into the plan.
- **D-03:** Correct-tier membership includes every page that documents a shipped, live surface:
  the seven CLI pages (`cli-usage`, `cli-council`, `cli-muster`, `cli-onboarding`,
  `cli-setup-check`, `cli-testing`, `cli-configuration`), `security-scanning.md`,
  `integration-tests.md`, `provider-expansion.md`, `sentinel.md`, `council.md`,
  `battalion-patterns-guide.md`, `sanctum-benchmarks.md`, `sanctum-migration.md`,
  `minio-file-repository-setup.md`, `redis-queue-adapter-setup.md`, `port-trait-template.md`,
  `release-automation.md`, `battalion-benchmarks.md`, and every non-appendix page in §5. An
  `L`-sized correct-tier row means the audit expects more than half the page to change; an `S`
  row means the cited lines only.
- **D-04:** PROJECT.md's Milestone 11 non-goal ("rewriting the 35 mdbook appendix files") is
  **not reopened and not edited**: ROADMAP Phase 35 SC1 ("every item in the Phase 34 mdBook work
  list is closed") is the binding statement for the 25 appendix IDs, and the D-01 tiering keeps
  the effort proportional — a banner is not a rewrite, and correct-tier pages are corrected to
  their findings, not rewritten for style. The SUMMARY and the CHANGELOG bullet say so; PROJECT.md
  is planning corpus and stays untouched (its own pending correction sits in the Phase 34 deferred
  register).
- **D-05:** MB-35 (`contributing/architecture-decisions.md` is an adapter-development guide under a
  nav slot titled "Architecture Decisions"): **retitle and add**. The existing file keeps its path
  (no inbound link breaks) and its `SUMMARY.md` entry is retitled "Adapter Development Guide"; a
  new short page `docs/src/contributing/adr-index.md`, titled "Architecture Decisions", is added
  directly after it — one table (ADR number, title, one-line decision, link) covering the ADRs that
  change what a crate consumer or operator sees (from `.planning/decisions/`: at least ADR-0033
  cargo-doc bar, 0037 `/v1` route surface, 0039 HTTP topology without Garrison/Arsenal, 0042
  LLM-native tool calling deferred, 0047 architecture appendix, 0048 `paladin-eval`, 0049
  Commissary, 0050 Treasurer reservation, 0051 token-economy versioning; the researcher confirms
  the list from the directory). Links to `.planning/decisions/*.md` use the GitHub blob URL form
  `migration-guide.md` already uses for `MIGRATION.md` (`follow-web-links = false`, so linkcheck
  does not fetch them). — **Reversibility:** reversible — one new page and one nav line.
- **D-06:** MB-16 (`api-reference/migration-guide.md`): correct the opening sentence ("up to the
  current v0.5.0 release") and the Timeline row marking `0.1.0` as Current to v0.10.0; the page
  keeps its pointer-only design and does not duplicate §9.1/§9.8 (`34-AUDIT.md` D-09 subsection).

### The superstep-engine page — MB-30, the one `L` item everything else links to
- **D-07:** Path `docs/src/user-guides/superstep-engine.md` (sibling naming — `control-flow.md`,
  `fault-tolerance.md`, `agent-runtime.md` carry no article; the audit's `the-superstep-engine.md`
  was explicitly a placeholder). Nav title **"WarEngine: Battlefield State & Superstep
  Execution"**, inserted in `docs/src/SUMMARY.md` immediately after `Maneuver Flow DSL` and before
  `Control Flow: Dynamic Routing & Subgraphs` — the position the audit assigned, and the one
  `control-flow.md`'s own forward reference implies. The page header carries the house marker in
  `platform-api.md`'s form: `**Since:** v0.10.0 (Phase 22)`. — **Reversibility:** costly — the
  path becomes a link target on at least four pages and in the CHANGELOG; renaming later is a
  multi-page edit.
- **D-08:** Scope is the audit's list, no more: `WarGraph` / `Battlefield` state and superstep
  merge semantics (ENG-01, SS-01); `Waypoint` full-snapshot checkpointing after every superstep
  and the `(ThreadId, WaypointId)` addressing scheme (ENG-03, SS-02); the three `WaypointPort`
  backends (ENG-05, SS-03); `EngineConfig` / `EngineLimits` (`max_supersteps`, `max_node_visits`,
  `run_timeout_secs`, `waypoint_durability`, `max_muster_tasks`) with their `APP_ENGINE_*` env
  overrides and `EngineError::RecursionLimitExceeded` (ENG-02, SS-05/SS-06);
  `WaypointRetentionService` (ENG-05, SS-07); and the graph-fingerprint scheme
  (`GRAPH_FINGERPRINT_VERSION`, `v6` today) and what bumps it. Routing, pause/resume, Aegis and
  middleware are **linked**, not re-explained — the page is the substrate the four existing guides
  build on and cross-links to each of them.
- **D-09:** Runnable snippets come from a **new** `crates/doc-examples/src/superstep_engine.rs`
  module, registered in `lib.rs`, with anchors for: building a small cyclic `WarGraph`, configuring
  `EngineLimits`, running it, and reading the resulting Waypoint history. Every such block is
  `{{#include}}`d and compile-verified by Layer 1; conceptual fragments (struct shapes, the
  fingerprint rule) are fenced `rust,ignore`. Mock adapters come from the existing `support.rs`
  module as an ordinary Rust import — it is not edited (D-26).
- **D-10:** The pages that today point forward to, or omit, the engine gain a link to the new page
  as part of their own IDs: `user-guides/control-flow.md` lines 29-30 (MB-22) replace "the full
  engine guide is future documentation (see the `Deferred` note in `23-CONTEXT.md`)" with the link
  and drop the planning-corpus reference; `introduction.md`'s nav index (MB-05);
  `architecture/overview.md` (MB-09); `architecture/domain-model.md`'s entities section (MB-11).
  MB-30 is therefore written in the first wave and the four dependents follow (Phase 34 D-21).

### Snippet verification and correction style — how SC3 is met on 59 pages
- **D-11:** One bright-line rule for every code block a corrected page touches: **(a)** a block
  that shows a complete flow or a constructor / method call against the live API on a Getting
  Started, User Guides, Architecture or Deployment Topologies page is moved into a `doc-examples`
  anchor and `{{#include}}`d (compile-verified); **(b)** a fragment — struct or trait excerpt,
  partial builder chain, config shape — is corrected to match the tree and fenced `rust,ignore`,
  and the page gains the existing header note if it lacks one ("a few illustrative fragments are
  marked `rust,ignore`", the `orchestration.md` line 16 / `content-processing.md` line 11
  wording); **(c)** appendix pages are corrected in place and fenced `rust,ignore`, and the
  executor proves each corrected sample compiles with the same throwaway
  `examples/_scratch.rs` + `cargo check --example _scratch --features <needed>` the audit used,
  records the command and result in the plan's SUMMARY, and never commits the scratch file.
  Rationale: SC3 requires compile verification for snippets "meant to run"; appendix pages are
  reference, not tutorials, and this keeps `doc-examples` growth bounded. — **Reversibility:**
  reversible — an `,ignore` fence can be promoted to an anchor later without touching prose.
- **D-12:** New `doc-examples` modules are limited to `superstep_engine.rs` (D-09) plus **one
  module per guide or architecture page whose §2 finding is signature-level**, named after the
  page (mirroring the D-18 include map convention): candidates are `paladin-agents.md`
  (`InMemoryGarrison::new(config)`, MB-27), `battalion-patterns.md` (five-argument
  `Commander::new`, MB-20), `hexagonal-design.md` (`LlmPort::generate(LlmRequest)`, MB-10),
  `arsenal-tools.md` (five-field `ArmamentResult`, MB-19), `herald-output.md` (the seven-method
  `Herald` trait, MB-24), `sanctum-vector-memory.md` (`RagRetrievalResult`, `ShedItem`,
  `with_token_counter`, the omission marker, MB-28), `design-patterns.md` (four-argument
  `PaladinExecutionService::new`, MB-12). The planner drops a candidate whose fix is a fragment
  under D-11(b).
- **D-13:** The systemic `paladin::paladin_ports::…` double-nesting defect (five pages:
  `minio-file-repository-setup.md`, `redis-queue-adapter-setup.md`, `sanctum-migration.md`,
  `port-trait-template.md`, `provider-expansion.md`) is fixed by **one import rule established
  once by the researcher** — the path that compiles for each port against the facade (the audit
  found compat re-exports for `paladin::core::…` and `paladin::application::services::…` but none
  for `paladin_ports`; the direct crate path `paladin_ports::output::…` is the expected answer) —
  and applied identically on every page. The relocated-with-no-shim adapter paths
  (`paladin::infrastructure::adapters::llm::…` on `provider-expansion.md` and `sentinel.md`) are
  rewritten to `paladin_llm::…` crate paths. Exit grep: `grep -rn 'paladin::paladin_ports::'
  docs/src` and `grep -rn 'paladin::infrastructure::adapters::llm::' docs/src` both empty.
- **D-14:** CLI family (MB-40..MB-46 plus `cli-usage`): every "Command Syntax" / "Command
  Options" block is **replaced by the live `--help` output** captured with
  `cargo run --features cli --bin paladin-cli -- <subcommand> --help`, in a ```text fence whose
  first line is the exact command, and every CLI page states the `--features cli` build
  requirement once (`Cargo.toml` `[[bin]] paladin-cli required-features = ["cli"]`). Fabricated
  flags and environment variables (`--mode`, `--synthesize`, `PALADIN_ENV_FILE`,
  `PALADIN_SKIP_VALIDATION`, the `-v`/`-q`/`--json` trio, …) are removed, not annotated. The
  capture command and output go in the SUMMARY so the block is re-derivable.
- **D-15:** Fabricated workflow YAML (`deployment/cicd.md` MB-31, `contributing/testing-guide.md`
  MB-36) is replaced by a **table of the real jobs** derived from `.github/workflows/ci.yml` and
  `release.yml` (job, what it gates, required or advisory per
  `.github/rulesets/protect-main-branch.json`), and any YAML kept is a verbatim excerpt of a real
  job captioned `# excerpt: .github/workflows/ci.yml — job: coverage`. `codeql.yml` joins the
  workflow listing worded exactly as `.github/instructions/security.instructions.md` words it
  (advisory-only, does not gate a merge). No invented job, no `.github/workflows/test.yml`.
  `scripts/check-doc-config.sh` must still pass on every YAML block. The same source-of-truth rule
  closes `security-scanning.md` (MB-57): the Snyk section states "evaluated and removed
  (2026-08-18), zero Rust coverage" and a CodeQL section states the advisory-only disposition,
  both taken from `security.instructions.md`, never re-argued.
- **D-16:** Corrections are **clean**: no new dated "Corrected 2026-09-…" callouts. Existing
  "Corrected 2026-08-24" callouts whose premise the fix supersedes (`monitoring.md` and
  `troubleshooting.md`'s "`opentelemetry` is not a dependency anywhere in the workspace",
  `performance-tuning.md`'s "no engine benchmark file") are rewritten to the current truth or
  removed; callouts that still hold (`kubernetes.md`'s "illustrative production manifests") stay.
  Provenance lives in the per-page commits (D-24) and the CHANGELOG entry, not in the prose.

### Vocabulary — SC4
- **D-17:** MB-02: `architecture/commissary.md` line 7 is reworded so the literal token is gone —
  "the rename rationale and the rejected-name list are in ADR-0049" — with **no allowlisted
  exception**; SC4 and Phase 30 D-14 are literal, and ADR-0049 remains the historical record of
  the old name. Exit grep `grep -rniE '\bQuartermaster\b' docs/src` is empty by construction.
- **D-18:** MB-04: `introduction.md`'s "Medieval Military Theme" table is **cut to a labelled
  excerpt** (at most eight terms, spelled exactly as `architecture/domain-model.md` spells them)
  with a sentence naming it an excerpt and linking `domain-model.md`'s table as the full in-book
  list. It is not expanded into a fourth complete list — the three ubiquitous-language lists stay
  three (`.github/copilot-instructions.md`, `.planning/PROJECT.md`, `domain-model.md`; Phase 34
  D-10), and only `domain-model.md` is a docs page.
- **D-19:** MB-03 / MB-11 (`architecture/domain-model.md`): the `GarrisonEntry` snippet is
  replaced with the live struct from `crates/paladin-core/src/platform/container/garrison.rs`
  (`id: Uuid`, `role: ConversationRole`, `content`, `timestamp: DateTime<Utc>`, `metadata`,
  `token_count: Option<u32>`, `is_summary: bool`), fenced `rust,ignore` with the source path in a
  comment; `token_count` here is a Garrison field, not a bare total, so no `TokenUsage` rewrite
  (the audit's disposition). The "Core Domain Entities" section gains `Battlefield`, `Waypoint`,
  `Aegis` and `TraceRecord` as short entries, each linking to its guide (the engine page,
  `fault-tolerance.md`, `operations/observability.md`).

### Version, MSRV, crate and feature-flag sweep — the S/M rows that recur across 20 pages
- **D-20:** Every dependency pin in a docs snippet reads the literal **`"0.10.0"`** (what
  `crates/doc-examples/Cargo.toml` uses and what SC3 measures against); every "Every code example
  targets the current **vX** workspace" sentence reads v0.10.0; every MSRV statement reads
  **1.88** (`Cargo.toml` `rust-version`, Phase 22.1); the `feature-flags.md` Dockerfile excerpt
  (`FROM rust:1.75`) is replaced by the real `Dockerfile` base image
  (`rust:1.93-slim-bookworm` today) or by a pointer to the real file. Historical tables are
  exempt — `migration-guide.md`'s Timeline, `build-baselines.md`'s snapshot toolchain,
  release-history rows — and each exemption is named in the exit-grep allowlist with a reason.
- **D-21:** Exit greps, run in the last plan and recorded verbatim in `35-EVIDENCE.md`, each
  empty or fully allowlisted: `grep -rnE '"0\.[5-9]\.[0-9]+"' docs/src` (version pins);
  `grep -rnE '\b1\.(70|75|85)(\.[0-9]+)?\b' docs/src` (MSRV drift); the D-13 and D-17 greps;
  `grep -rn 'test\.yml\|build-release' docs/src` (the fabricated CI names); and the Phase 31 D-29
  `token_count`-beside-`TokenUsage` re-check on its ten pages. An allowlist entry names the file,
  line and why it is historical.
- **D-22:** Feature-flag and crate tables (`feature-flags.md` MB-15, `installation.md` MB-06,
  `architecture/crate-map.md` MB-13, `api-reference/crate-map.md` MB-14, `architecture/overview.md`
  MB-08, `stable-api.md` MB-17, `build-baselines.md` MB-39) are **regenerated from
  `[features]` of the facade `Cargo.toml` and each crate's `Cargo.toml`** — every shipped flag
  present once with its crate and what it gates (`otel`, `dev-ui`, `redis-cache`,
  `storage-postgres`, the six Phase 17 providers, `vision`, `content-processing`, `web-server`,
  `notifications`, `qdrant`, …); crate lists name all eleven library crates plus the facade
  (`paladin-eval` and `paladin-herald` added everywhere they are missing), and the mermaid crate
  graph on both crate-map pages gains the `mem --> llm` edge (Phase 33 D-03) so diagram and prose
  agree. `stable-api.md`'s catalogue paths move from the pre-workspace `paladin::core::…` layout to
  the live crate paths, checked against the files the audit names.

### Closure evidence, commits, CHANGELOG, and the Phase 36 boundary
- **D-23:** Every plan's SUMMARY carries a **closure table**: `MB-nn | page | disposition
  (corrected / archived / new / retitled) | commit | how the §5 finding was addressed`. The final
  plan writes **`35-EVIDENCE.md`** (the `NN-EVIDENCE.md` house pattern): the full `docs.yml`
  sequence run locally with tool versions against the pins, the `git status --porcelain -- docs`
  check after `mdbook-mermaid install`, the linkcheck summary line, both scripts' result lines,
  `make api-surface`, and the D-21 exit greps, all verbatim. VERIFICATION cites IDs against
  commits (Phase 34 D-03). After each page fix the executor may re-run
  `.planning/phases/34-documentation-currency-audit/34-signals.sh <page>` as a before/after
  self-check; the audit's §2 producing command for that row is the proof the finding is gone.
- **D-24:** Commit granularity is **one commit per page** (or per tightly-coupled pair, such as the
  two crate-map pages), subject `docs(35): <what changed> (MB-nn[, MB-mm])`, so
  `git log --oneline --grep 'MB-'` reproduces the closure map mechanically. A new `doc-examples`
  module lands in the same commit as the page that includes it. Commits use plain
  `git add -- <files> && git commit -q -m … -- <files>` with a long timeout (the pre-commit hook
  runs workspace clippy whenever `.rs` or `Cargo.toml` files are staged; the GSD commit helper
  times out under it).
- **D-25:** `CHANGELOG.md` `[0.10.0]` gains a **`### Documentation`** subsection (precedent: the
  0.5.0 entry at CHANGELOG line ~1062), placed after `### Fixed` and before `### Known
  limitations`, reader-facing: one bullet for the new engine guide, one bullet per nav section
  summarising the corrected pages, one bullet naming the archived appendix pages and their live
  replacements. **No `MB-nn` IDs in the CHANGELOG** — they are planning-corpus identifiers. Phase 36
  appends its own bullets under the same heading; Phase 35 creates it. — **Reversibility:**
  reversible until Phase 37 tags; after the tag the entry is part of the release record.
- **D-26:** Phase 36 boundary in the shared `crates/doc-examples` crate: Phase 35 **only adds
  modules** and registers them in `lib.rs`; it never edits an existing module's anchors or
  `support.rs` (those are `EX-nn` items). If a page correction needs an existing anchor changed,
  Phase 35 records it in its own phase-local `deferred-items.md` (Phase 34 D-19 pattern) as a
  Phase 36 pointer and leaves the include in place. `examples/README.md` is Phase 36's. The
  api-surface baseline does not cover `doc-examples` (`publish = false`; zero matches in
  `.project/current-exports.txt`), so `make api-surface` is expected to report no change and is
  still run as the last gate; a genuinely required public change would follow the Phase 36 SC5 /
  Phase 33 pattern (`MIGRATION.md` §9.2 row plus allowlist), but none is anticipated.
- **D-27:** The Phase 34 §2 rows remain the evidence base; Phase 35 re-runs a row's producing
  command only to prove its fix, never to re-audit a `current` page. A defect noticed on a
  `current` page while editing a neighbour is recorded in the phase-local `deferred-items.md`
  with a proposed classification, not fixed silently (Phase 29 D-12 / Phase 34 D-00c).

### Claude's Discretion
- Plan count and waves. The natural shape is four: wave 1 — MB-30 plus its `doc-examples` module
  and the nav insertion; wave 2 — Getting Started / User Guides / Architecture in parallel with
  API Reference / Contributing / Deployment / Operations; wave 3 — the 25 appendix rows (CLI
  family as one plan); wave 4 — CHANGELOG, exit greps, `35-EVIDENCE.md`. The planner may merge or
  split so long as MB-30 lands before its four dependents.
- Exact banner wording for archive-tier pages (must name the live source and, where one exists,
  the ADR), and the exact ADR set in the index page (D-05) after reading `.planning/decisions/`.
- Archive vs correct tier for `sanctum-benchmarks.md`, `battalion-benchmarks.md`,
  `release-automation.md` and any other appendix page D-02/D-03 do not name.
- Whether the three superseded "Corrected 2026-08-24" callout removals (D-16) ride in their page's
  commit or in one small commit.
- Whether the ADR index page (D-05) is the last nav entry in Contributing or sits directly after
  the retitled adapter guide — the audit assigned no position because the page did not exist.

### Folded Todos
None — both matched todos are keyword false positives (see Reviewed Todos below).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The phase and its work list
- `.planning/ROADMAP.md` §"Phase 35: mdBook Currency" — goal, five success criteria, sources;
  §"Phase 36" and §"Phase 37" — the boundary (rustdoc, examples, release re-seal are theirs).
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §5 — the 60-row work list
  (ID, classification, size, location, cites, blocks, evidence anchor); §2 — the per-page findings
  every fix must address, plus the subsections this phase consumes directly: "Superstep-engine
  dedicated-page decision" (MB-30 scope and nav position), "Vocabulary sweep" (MB-02, MB-03),
  "D-09 subsection" (upgrading.md already agrees — no edit), "Build baseline" (the green baseline
  and tool versions), "D-18 — `doc-examples` module → page include map" (which module a page
  edit re-renders); §7 — deferred routing.
- `.planning/phases/34-documentation-currency-audit/34-CONTEXT.md` — D-00a…g, D-03, D-04, D-06,
  D-10, D-11, D-18, D-19, D-21, D-22, D-23 (all carried forward above).
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — the exact commands and
  captures behind every §2 row (re-run to prove a fix, D-23/D-27).
- `.planning/phases/34-documentation-currency-audit/34-signals.sh` and `34-shipped-tokens.txt` —
  the per-page nine-signal-class runner, reusable as a before/after self-check.
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` — the five entries this
  phase must not absorb.
- `.planning/REQUIREMENTS.md` — `CURR-01…05` and the prefix protocol (D-00g).

### The gate being kept green
- `.github/workflows/docs.yml` — the required "Build MDBook" check: tool pins (`mdbook 0.4.40`,
  `mdbook-mermaid 0.13.0`, `mdbook-linkcheck 0.7.7`), `mdbook-mermaid install docs/`, both scripts,
  the `docs/book/html/` upload path.
- `docs/book.toml` — `[output.linkcheck] warning-policy = "error"`, `follow-web-links = false`.
- `scripts/check-doc-examples.sh` — Layer 1 compile gate on `crates/doc-examples`, Layer 1b README
  mirror, Layer 2 inline-block scan (what `rust,ignore` and `fn main` mean to it).
- `scripts/check-doc-config.sh` — every ```yaml block must parse.
- `crates/doc-examples/src/lib.rs`, `crates/doc-examples/Cargo.toml`, `crates/doc-examples/src/support.rs`
  — module registration, available features, shared mock adapters (D-09, D-26).
- `docs/src/SUMMARY.md` — the nav; MB-30 inserts before line 25 (`control-flow.md`); D-05 adds one
  Contributing entry.
- `Makefile` targets `check-doc-examples`, `check-doc-config`, `api-surface`.

### Sources of truth the corrections are made toward
- `Cargo.toml` `[workspace.package] version`, `rust-version`, `[features]`, `[[bin]] paladin-cli
  required-features`, `[[example]]` entries; each `crates/*/Cargo.toml` `[features]` (D-20, D-22).
- `rust-toolchain.toml`; `Dockerfile` (base image, D-20).
- `src/bin/paladin-cli.rs` — the `Commands` enum; the `--help` capture source (D-14).
- `.github/workflows/ci.yml`, `release.yml`, `codeql.yml`; `.github/rulesets/protect-main-branch.json`
  (required vs advisory, D-15).
- `.github/instructions/security.instructions.md` — the Snyk removal and CodeQL advisory-only
  wording `security-scanning.md` must repeat (D-15).
- `crates/paladin-core/src/platform/container/garrison.rs` (`GarrisonEntry`, D-19);
  `crates/paladin-core/src/platform/container/{battlefield,waypoint,aegis,trace}.rs` (D-19);
  `crates/paladin-ports/src/output/llm_port.rs` (`LlmPort::generate`, MB-10);
  `crates/paladin-battalion/src/commander.rs` and `council_service.rs` (MB-20, MB-48);
  `crates/paladin-memory/src/services/rag_retrieval_service.rs` and
  `src/application/services/paladin/paladin_execution_service.rs` (`RagRetrievalResult`,
  `format_retrieved_context`, the omission marker, MB-28);
  `crates/paladin-core/src/platform/container/arsenal/core.rs` (`ArmamentResult`, MB-19);
  `crates/paladin-core/src/platform/container/herald.rs` (the `Herald` trait, MB-24).
- The engine surface for MB-30: `crates/paladin-core/src/platform/container/` engine, battlefield
  and waypoint modules, `EngineConfig`/`EngineLimits` and the `APP_ENGINE_*` bindings, the three
  `WaypointPort` adapters, `WaypointRetentionService`, `GRAPH_FINGERPRINT_VERSION` — located by
  the researcher from `.planning/phases/22-*/22-CONTEXT.md` and the `34-AUDIT.md` §1 Phase 22 /
  22.1 rows (SS-01…SS-08).
- `MIGRATION.md` §9.1 and §9.8 — already agree with `upgrading.md` (D-00f); read only.
- `CHANGELOG.md` `[0.10.0]` (the shipped-surface narrative) and the 0.5.0 `### Documentation`
  precedent near line 1062 (D-25).

### Decision records and prior-phase decisions that shape the edits
- `.planning/decisions/0047-architecture-appendix-disposition.md` — the archive-banner pattern and
  the "retain, do not delete" rule (D-01, D-02); `docs/src/appendix/design-and-architecture.md`
  lines 3-9 — the banner as shipped.
- `.planning/decisions/0049-commissary-design-and-rename.md` — the rename record MB-02 points at
  (D-17); `0033-cargo-doc-warning-bar.md` — what `doc-coverage-report.md`'s banner points at (D-02);
  `0048-paladin-eval-composition-crate.md` — the crate every crate list is missing (D-22).
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md` — D-01
  vocabulary rule, D-06 (commissary.md sketch mirrors a unit test), D-14 exit grep.
- `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` — D-08 bare-count rule, D-29 hit list.
- `.planning/phases/32-unified-token-primitives/32-CONTEXT.md` — D-12 doc sweep for the removed
  `TokenCounter` pair (already applied; do not reintroduce the names).
- `.planning/phases/33-commissary-in-tree-adoption/33-CONTEXT.md` — D-03 (crate-graph docs follow
  the `mem --> llm` edge), D-12 (`RagRetrievalResult` shape), D-15 (the two markers).
- `.github/copilot-instructions.md` naming table, `.planning/PROJECT.md` term table,
  `docs/src/architecture/domain-model.md` — the three ubiquitous-language lists (D-18).
- `.planning/PROJECT.md` line ~846 — the Milestone 11 appendix non-goal (not reopened, D-04).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`crates/doc-examples` + `scripts/check-doc-examples.sh` Layer 1** — the compile-verification
  mechanism SC3 names; ten modules and ~40 anchors already back ten pages (the D-18 include map).
  Adding a module is: file under `src/`, `pub mod` line in `lib.rs`, anchors, `{{#include}}` on
  the page. `support.rs` supplies mock adapters and constructors as an ordinary import.
- **ADR-0047 archive banner** (`appendix/design-and-architecture.md` lines 3-9) — the exact
  blockquote shape for archive-tier pages.
- **`34-AUDIT.md` §2 rows** — every stale row names the producing command that found the defect;
  re-running it after the fix is the row's own proof. `34-signals.sh <page>` re-runs all nine
  signal classes for a page in one call.
- **`Since:` marker** — `**Since:** v0.10.0 (PRD 06, Phase 27)` on `platform-api.md` line 3 is the
  form the new engine page copies.
- **`migration-guide.md` lines 7-14** — the GitHub blob-URL form for linking root-level or
  `.planning/` files without linkcheck fetching them.
- **`orchestration.md` line 16 / `content-processing.md` line 11** — the header sentence that
  declares `rust,ignore` fragments illustrative.

### Established Patterns
- 368 ```rust,ignore and 248 bare ```rust blocks exist under `docs/src`; Layer 2 checks only bare
  blocks that contain `fn main` and import no external crate — in practice nothing outside
  `doc-examples` is compiled, which is why D-11 is a rule rather than a script change.
- Phase 16's "Corrected 2026-08-24" callouts were the previous correction style; the audit found
  two whose premise has since become false — the reason D-16 retires the style.
- Commit scope is `docs(NN)` for docs-only commits (`docs(33): …`, `docs(32): …` in
  `git log -- docs/src`); the pre-commit hook runs workspace clippy when `.rs`/`Cargo.toml` are
  staged (~2 min warm), which `doc-examples` commits will trigger.
- The `doc-examples` crate is `publish = false`, absent from `.project/current-exports.txt`, and
  built with `web-server`, `openai`, `mock`, `llm`, `news-api`, `redis-queue` features — enough for
  an engine example using mock adapters.

### Integration Points
- `docs/src/SUMMARY.md` — one insertion for MB-30, one retitle plus one insertion for MB-35.
- `crates/doc-examples/src/lib.rs` — `pub mod` registrations for each new module (shared with
  Phase 36, which edits existing modules; D-26).
- `CHANGELOG.md` `[0.10.0]` — the new `### Documentation` heading Phase 36 appends to (D-25).
- `.github/workflows/docs.yml` "Build MDBook" is a **required** status check on `main`; every
  commit on the branch keeps it green.
- Phase 37 re-seals the Phase 29 gates on the final post-documentation commit; `35-EVIDENCE.md`'s
  final `docs.yml` run is the docs half of that "before" reference.

</code_context>

<specifics>
## Specific Ideas

- Archive banner text, adapted from ADR-0047's: `> **Archived — historical document.** This page
  records <what it was> as of <date/milestone> and is not maintained. For the current, maintained
  <subject>, see <live page>. <One sentence naming the disposition record, e.g. ADR-0033 / the
  Phase 34 audit>.`
- CLI page capture caption: the first line of each ```text block is the literal command, e.g.
  `$ paladin-cli council --help`, followed by the verbatim output; the page's build note reads
  `cargo build --release --features cli --bin paladin-cli`.
- Commit subject examples: `docs(35): write the WarEngine superstep-engine guide (MB-30)`,
  `docs(35): regenerate feature-flag tables from Cargo.toml (MB-15)`,
  `docs(35): archive user-system and user-rest-api behind ADR-0047 banners (MB-59, MB-60)`.
- Exit-grep allowlist row shape in `35-EVIDENCE.md`: `file:line | matched text | reason
  (historical table / release history / snapshot toolchain)`.
- The engine page's `{{#include}}` blocks should read top-to-bottom as one small program: build
  the graph, set limits, run, inspect the waypoints — the same narrative `fault-tolerance.md`
  follows with its eleven anchors.

</specifics>

<deferred>
## Deferred Ideas

- **Regenerating `doc-coverage-report.md` from a real measurement** — archived instead (D-02);
  Phase 36 owns the rustdoc bar, and a later docs pass can regenerate once that bar is green.
- **Existing `doc-examples` anchors that a page fix would like to change** — Phase 36 `EX-nn`
  territory; Phase 35 records each in its phase-local `deferred-items.md` (D-26).
- **Moving ADRs into `docs/src/` (an ADR chapter rather than an index of GitHub links)** —
  backlog; D-05's index page is the minimal version.
- **A script that regenerates feature-flag tables from `Cargo.toml`** — D-22 does it by hand this
  phase; automating it is a tooling idea for a later milestone.
- **PROJECT.md corrections** (the Milestone 11 non-goal wording; the crate-examples claim) —
  planning corpus, already in the Phase 34 deferred register.
- **`make doc` failing on rustdoc warnings / a rustdoc gate in `make clean-code`** — Phase 36 SC2.

### Reviewed Todos (not folded)
- **Verify local `make coverage` reproduces CI's 82.39% figure**
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6) — matched
  on the generic keywords "docs, src, github, workflows, yml". Its documentation slice was already
  folded and closed by Phase 34 (MB-36 carries the coverage-command comparison; this phase corrects
  `testing-guide.md` under that ID). The remainder — the end-to-end Docker-machine run — is
  infrastructure verification, not a docs item, and stays the maintainer's pending todo (Phase 34
  `deferred-items.md` §"Plan 34-04, Task 2"). The mechanical ≥ 0.4 auto-fold rule was deliberately
  not applied.
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.6) —
  matched on "yml, github, workflows, crates" only; Phase 34's object-store sweep found every
  MinIO image pin in `docs/src` already current (zero `MB-nn`), and the evaluation itself is an
  infrastructure decision with no page to correct. Not folded, same call as Phases 32, 33 and 34;
  stays pending with no `resolves_phase` tag.

</deferred>

---

*Phase: 35-mdbook-currency*
*Context gathered: 2026-09-17*
