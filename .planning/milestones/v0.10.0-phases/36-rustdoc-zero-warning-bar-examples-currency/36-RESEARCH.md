# Phase 36: Rustdoc Zero-Warning Bar & Examples Currency - Research

**Researched:** 2026-09-17
**Domain:** Rust documentation tooling (rustdoc lint bars, intra-doc links, doctest harness,
`examples/` gallery currency, CI/gate wiring)
**Confidence:** HIGH — every quantitative claim below was re-measured against the current tree
in this session (toolchain `cargo 1.97.1` / `rustc 1.97.1`, HEAD `619551a781f6e352fac896d5fab1ba8630876585`),
not carried from `34-AUDIT.md` unverified.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Inherited — locked by earlier phases, not re-litigated
- **D-00a:** The bar is **zero `warning:` lines** on `cargo doc --workspace --no-deps`, ratified
  by `.planning/decisions/0033-cargo-doc-warning-bar.md` and enforced by the required `lint` job
  (`ci.yml:62-63`). The second bar is `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --all-features --no-deps` exiting 0 (Phase 34 D-12). Neither is reopened or weakened.
- **D-00b:** The work list is `34-AUDIT.md` §6 — 143 `RD-nn` rows in 75 location groups (63
  groups carry a follower closed by the same fix) and 64 `EX-nn` rows (5 stale existing items +
  59 gap rows). IDs are stable, never renumbered (Phase 34 D-03); the phase may re-batch rows
  into its own plans and waves but must close every ID (Phase 34 D-21). SUMMARY/VERIFICATION
  cite closures by ID.
- **D-00c:** Shipped tree outranks any document (Phase 34 D-00g). A doc link that points at
  something the tree does not expose is corrected toward the tree, never the reverse.
- **D-00d:** `crates/doc-examples` is out of scope for the `missing_docs` bar by recorded
  disposition (ADR-0033 amendment, 2026-08-24); it is still a workspace member that
  `cargo doc --workspace` builds, so any rustdoc *link* warning it emits counts against the bar.
- **D-00e:** Phase 35 D-26 boundary: Phase 35 only *added* `doc-examples` modules
  (`arsenal_tools`, `battalion_patterns`, `herald_output`, `paladin_agents`,
  `sanctum_vector_memory`, `superstep_engine`). Edits to existing module anchors, to
  `support.rs`, and to `examples/README.md` are Phase 36's. Phase 35's register recorded **no**
  pending anchor-change pointer (`35-mdbook-currency/deferred-items.md` §"Plan 35-01, Task 3"
  item 2), so no extra `EX-nn` is inherited from it.
- **D-00f:** Doctests are green at baseline — `cargo test --workspace --doc` under default
  features: 462 passed / 0 failed / 210 ignored (`34-AUDIT.md` §3 "Doctest baseline"). SC3
  requires this stays green; the wider `--all-features` doctest invocation is **not** run because
  it trips the unrelated `cli_isolation` conflict Phase 36.1 SC3 owns.
- **D-00g:** The `# Examples`-heading rule applies to the frozen 76-item entry-point set only
  (Phase 16 D-05/D-06, Phase 34 D-00e). Its 76→101 drift and 19 MISSING items are a finding
  about the rule's apparatus, dispositioned by **Phase 36.1 SC2**, not here (D-13 below).
- **D-00h:** Requirement prefix is `CURR-*` (Phase 34 D-20). `CURR-01…05` are Phase 34's; Phase
  35 minted its own; the planner mints the next numbers for Phase 36 in
  `.planning/REQUIREMENTS.md`.
- **D-00i:** Vocabulary rules hold in every new example and every rewritten doc line: Phase 30
  D-01 (domain roles use the Medieval-military term), `Quartermaster` never appears (Phase 30
  D-14), no bare token total beside a `TokenUsage` split (Phase 31 D-08).
- **D-00j:** `CHANGELOG.md` `[0.10.0]` already carries a `### Documentation` subsection (Phase 35
  D-25). Phase 36 **appends** its bullets there — reader-facing, no `RD-nn`/`EX-nn` IDs.

#### Re-baseline before fixing — the D-23 rule is triggered
- **D-01:** The audit's 143 rows were measured at `ee1fb160f8e743e638b32beb6c4e32be4ede9325`.
  Verified 2026-09-17 at discuss time: `git diff --stat ee1fb160…..HEAD -- src crates examples
  Cargo.toml ':!crates/doc-examples'` touches **only `Cargo.lock` (+1 line)** — no library source
  moved — but `crates/doc-examples` gained six modules and a `Cargo.toml` edit (557 insertions),
  and that crate is part of the `--workspace` doc build. The Phase 34 D-23 re-run rule is
  therefore **triggered**. The researcher re-runs, on the current HEAD and records verbatim in
  `36-RESEARCH.md`: (1) the `ci.yml:63` command; (2) the workspace `-D warnings --all-features`
  command; (3) the per-crate `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features
  --no-deps` sweep across the eleven library crates, the facade and `paladin-doc-examples`
  (Phase 34 D-14); (4) `cargo test --workspace --doc`; (5) the four `ci.yml:548-558` example
  invocations plus `scripts/check-doc-examples.sh`. The result is diffed against §6 row by row.
- **D-02:** **The bar is zero, not "the 143 rows."** Any `warning:` or `-D warnings` error the
  re-measure finds that has no §6 row is minted as a new row **`RD-144` onward** (and `EX-123`
  onward for examples) in a `## Re-baseline delta` section of `36-RESEARCH.md`, with the same
  columns as §6 (ID, size, file:line, kind, message first line, evidence anchor), and closed in
  this phase exactly like an audit row. Any §6 row that no longer reproduces is recorded as
  `closed-by-drift` with the command output that proves it, never silently skipped. —
  **Reversibility:** costly — Phase 36.1 SC4 and Phase 37 verify by ID against this phase's
  closure table; a row that is neither closed nor dispositioned breaks that chain.
- **D-03:** Toolchain: the devcontainer carries cargo 1.97.1; the CI lint job's toolchain is what
  `ci.yml` pins. If the local zero and the CI count disagree on the closing commit, the CI figure
  wins (it is the gate) and the difference is recorded in `36-CI-EVIDENCE.md`, not explained away.

#### Rustdoc fix technique — one rule per warning kind (SC1, SC2, SC5)
- **D-04:** The 143 rows are **all link/HTML defects**, counted from `34-AUDIT.md` §3: 75
  unresolved links, 57 private intra-doc links, 6 redundant explicit links, 4 unclosed HTML tags,
  1 broken intra-doc link. There are **zero `missing_docs` rows** — the ROADMAP goal's "every
  public item Phases 22-33 added or changed has rustdoc" is already satisfied by the uniform
  `#![warn(missing_docs)]` posture plus a zero-warning run. No new rustdoc prose is written to
  satisfy SC1; prose is touched only where a link fix needs a rewording. All rows are sized S
  (Phase 34 D-04).
- **D-05:** **Private intra-doc link (57 rows):** replace the link with plain code font
  (`` `Commander::analyze_and_select` ``) or reword to name the public entry point that reaches
  it. **Never** widen visibility (`pub(crate)` → `pub`) and **never** add
  `#[allow(rustdoc::private_intra_doc_links)]` — the first moves the api-surface baseline
  (SC5), the second hides the defect the bar exists to catch. — **Reversibility:** reversible —
  a de-linked mention can be re-linked in one line if the target is later made public on its own
  merits.
- **D-06:** **Unresolved link (75 + 1 rows):** resolve to the item's real path with an explicit
  intra-doc target (`` [`WarGraph`](crate::engine::WarGraph) `` or the shorthand `` [`WarGraph`] ``
  when it is in scope of the doc'd item). Where the target lives in another crate, use that
  crate's path (`` [`TokenUsage`](paladin_core::TokenUsage) ``) — the crate is already a
  dependency or the mention is wrong. Where the target does not exist in the tree (a renamed or
  removed item), the mention is corrected to the shipped name per D-00c, with the §9.2 register
  (`MIGRATION.md` §9.2) as the rename source.
- **D-07:** **Feature-gated targets:** the all-features sweep found 77 errors against the
  default run's 65 because some link targets sit behind `cfg(feature = …)`. Every fix must hold
  under **both** the default and `--all-features` builds. Rule: a doc comment may link to a
  feature-gated item only when the doc'd item is gated by the same (or a narrower) feature;
  otherwise the mention is plain code font. No `#[cfg_attr(docsrs, …)]` machinery is introduced
  this phase (a `docs.rs` metadata build is out of scope).
- **D-08:** **Redundant explicit link (6 rows):** collapse `` [`Foo`](Foo) `` to `` [`Foo`] ``.
  **Unclosed HTML tag (4 rows):** wrap the generic (`Vec<T>`, `Arc<dyn Port>`) in backticks; do
  not escape with `&lt;`.
- **D-09:** **Lead-row discipline:** each of the 75 location groups is fixed once, at the lead
  row's file:line; the closure table lists the lead ID and every ID in its `Blocks` cell as
  closed by the same commit. A row whose fix turns out to need a *different* line than the audit
  cited records the actual line beside the cited one — the cited line is the audit's contract,
  the actual line is the fix's evidence.
- **D-10:** Fixes are verified per crate as they land — `RUSTDOCFLAGS="-D warnings" cargo doc
  -p <crate> --all-features --no-deps` and `cargo doc -p <crate> --no-deps 2>&1 | grep -c
  'warning:'` — so parallel worktrees per crate (the Phase 35 wave pattern) can each prove their
  own crate green without waiting for the workspace run.

#### Gate wiring — where the commands live so the count cannot regrow (SC2, SC3)
- **D-11:** A **new `make doc-check` target** is the single local source of truth. It runs, in
  order: the `ci.yml:63` command verbatim (default features, `tee` + `! grep -q "warning:"`),
  then `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`, then
  `cargo test --workspace --doc`. `clean-code` gains `doc-check` as a dependency
  (`clean-code: fmt lint lint-shell check doc-check`). A **pre-push** hook entry `doc-check`
  is added to `.pre-commit-config.yaml` beside `check-api-surface`, with the same
  `files: ^(src|crates)/.*\.rs$|^Cargo\.toml$` filter and `stages: [pre-push]`, and a comment in
  the same voice explaining why it sits at pre-push (cost, and GSD executor commits bypass the
  commit stage). The existing `make test-doc` target stays; `make doc` (with `--open`) stays.
  — **Reversibility:** reversible — Makefile and hook config are one-commit edits either way.
- **D-12:** The **CI lint job** gains one step directly after "Check documentation" (`ci.yml:62-63`),
  named "Check documentation (all features, -D warnings)", running the second D-00a command.
  This is a change to a required check, so it follows the Phase 33 pattern: a real CI run on the
  pushed branch is recorded in `36-CI-EVIDENCE.md` (job, run id, step result) before the phase is
  verified. `cargo test --workspace --doc` already runs in CI (`ci.yml:499`) and is not
  duplicated. — **Reversibility:** costly — once merged, every PR gates on it; removing it later
  is a visible weakening of the bar that needs its own ADR-0033 amendment.
- **D-13:** **Ordering:** gate wiring (D-11, D-12) lands in the **last wave**, after the
  workspace zero is measured on the branch. Wiring it earlier would turn `make clean-code` and
  every push red for the whole phase. The Makefile/hook/ci.yml edits and the closing
  measurement are one commit, so `git bisect` never lands on a commit where the gate exists but
  fails.
- **D-14:** `scripts/check-all-examples.sh` currently runs `cargo check --example <name>
  --all-features` per file, which hides exactly the required-features gap `ci.yml:538-546`
  documents. It is rewritten to run the **four `ci.yml:548-558` invocations verbatim** (minus
  `--offline` when the devcontainer's registry cache requires it, recorded) plus the binary-count
  assertion, so the local examples check and CI's "Example Muster" agree. It is **not** added to
  pre-push (four full example builds are too slow for a push hook); it stays a `make`-invoked
  check (`make check-examples`, new target) that the executor runs and records before the closing
  commit. — **Reversibility:** reversible.

#### Example gap list — 59 rows, ≈10-14 programs (SC4)
- **D-15:** A gap row is a **capability**, not a file. New programs are organised **per
  capability cluster**, each a runnable story, and **every `EX-nn` gap row maps to exactly one
  named program and one `examples/README.md` section whose "Demonstrates" line names the
  capability**. The planner fixes the exact list; the clusters the audit's gap list implies are:
  - **WarEngine configuration & checkpoints** — EX-62 (custom `WaypointPort` backend), EX-63
    (inspecting the waypoints table), EX-64 (`EngineConfig`), EX-65
    (`APP_ENGINE_MAX_SUPERSTEPS`), EX-66 (`WaypointRetentionService`), EX-80 (fingerprint version
    printed and explained).
  - **Control flow** — EX-67 (`EdgeCondition::Custom` fail-closed), EX-68 (`NodeSpec::Battalion`
    nesting), EX-69 (`LlmDecisionEvaluator` / `StrategySelection::Semantic`), EX-70
    (`APP_ENGINE_MAX_MUSTER_TASKS`).
  - **Human-in-the-loop** — EX-71 (`NodeSpec::Gate`), EX-72 (`resume_with`), EX-73
    (`ChronicleService` replay/fork).
  - **Graceful shutdown** — EX-74, EX-75, EX-76 (`ShutdownCoordinator` and the two env vars).
  - **Platform API client** — EX-77, EX-78, EX-79 (thread state/resume/history routes), EX-91
    (`POST /v1/runs`), EX-92 (SSE stream), EX-93 (cancel), EX-94 (assistants), EX-95
    (schedules), EX-96 + EX-97 (webhook signature verification and the SSRF override, receiver
    side), EX-98 (`RunQueuePort`), EX-99 (`APP_RUN_STORE_BACKEND`), EX-104 (dev-ui route).
    Programs in this cluster start the shipped axum app **in-process** (the
    `http_service_host.rs` / `doc-examples/http_service_host.rs` pattern) and drive it with
    `reqwest`, so they run without an external server; they are `[[example]]` targets gated on
    `web-server` (D-17).
  - **Node-result cache** — EX-81 (`redis-cache` feature), EX-82 (`APP_NODE_CACHE_ENABLED`).
  - **Agent runtime & middleware** — EX-83 (custom `ExecutionMiddleware`), EX-84
    (`AgentRuntimeConfig`), EX-85 (custom `TokenCounterPort`), EX-86 (`HistoryTrimmer` +
    `SummarizationMiddleware`), EX-87 (`VaultPort` / `ConfinedVault`), EX-88 + EX-90 (structured
    output + `schemars`), EX-89 (`tool_error_mode = FailRun`).
  - **Observability & eval** — EX-100 (`TraceRecord` / `TraceEvent`), EX-101 (`TraceConfig`),
    EX-102 + EX-103 (`PALADIN_TRACE_OTEL_ENABLED` and the `otel` feature), EX-108 (`run_traces`
    table), EX-105 (`paladin-eval` + `eval_scenarios!`), EX-106 (`PALADIN_EVAL_LIVE`), EX-107
    (`paladin-cli eval run <glob>` — a program that shells out or a README section pointing at
    the CLI with a checked-in scenario file; the planner decides which satisfies "runnable").
  - **Token economy** — EX-109 (`Commissary` direct use), EX-110 (`TokenUsageResponse` on the
    HTTP surface — may live in the Platform API client), EX-111 (Anthropic `prompt_tokens`
    including cache tokens — demonstrated with the mock adapter's usage shape and a comment,
    since no live key runs in CI), EX-112 (`is_exact`), EX-113 (`Commissary::new` without the
    removed argument), EX-114 (`resolve_context_window`), EX-115 (`WindowSource` /
    `WindowFallbackPolicy` / `ResolvedWindow` from the facade).
  - **RAG retrieval** — EX-116 (`RagRetrievalResult`), EX-117 (`shed: Vec<ShedItem>`), EX-118
    (`RagRetrievalError`), EX-119 (`retrieve_context_with_timeout`), EX-120
    (`with_token_counter`) — extends or sits beside the existing `paladin_with_rag.rs`
    (EX-40, current); the planner decides whether to extend that file or add a sibling, but
    every one of the five IDs maps to a named program.
  — **Reversibility:** costly — program names become README anchors and CHANGELOG bullets;
  renaming after the phase is a multi-file edit.
- **D-16:** Every new program is **runnable offline where the capability allows**: LLM calls go
  through `paladin_llm`'s `mock` feature adapter (`MockLlmAdapter`, already a dev-dependency
  pattern in `paladin-battalion` and used by `doc-examples/support.rs`), memory and waypoints use
  the in-memory ports, and the program prints what it demonstrates. Programs that genuinely need
  an external service (Redis for EX-81/EX-98, an OTLP endpoint for EX-102/EX-103) **build** under
  the CI split and state the prerequisite in their README section and file header; they are not
  run in CI. The executor runs each offline program once (`cargo run --example <name>`), and
  records the exit code and the first lines of output in `36-EVIDENCE.md`.
- **D-17:** **Feature gating:** default features first. A program is declared `[[example]]` with
  `required-features` in the root `Cargo.toml` (lines 444-462 are the template) **only** when the
  capability *is* the feature (`web-server` for the Platform API cluster, `redis-cache` for
  EX-81, `otel` for EX-103, and any other gate the tree imposes). Each new gated target gets its
  own `cargo build --example … --features …` line in `ci.yml`'s "Example Muster" job **and** in
  `scripts/check-all-examples.sh` (D-14), because the bulk selector silently skips it. The
  `ci.yml:538` comment's file count and target count are updated in the same edit (they change
  anyway); Phase 36.1 SC2 then *verifies* the comment rather than re-doing it.
- **D-18:** No example is deleted. The audit found all 48 programs, all `doc-examples` modules
  and `live_vendor_smoke.rs` building green and 58 of them `current`; there is no obsolete
  program to remove. If the re-baseline (D-01) finds one, it is updated to the shipped API rather
  than deleted; deletion with a `CHANGELOG.md` note (ROADMAP SC4's escape hatch) is reserved for
  a program whose capability no longer exists in the tree, and none is known.

#### Stale existing examples and the README (EX-01, EX-33, EX-55, EX-121, EX-122)
- **D-19:** **EX-33 / EX-55** (`examples/http_service_host.rs` and its `doc-examples` sibling
  claim server parity but do not mount `thread_router` (Phase 24) or `run_router` (Phase 27)):
  both are updated to mount the routers the shipped `paladin-server` binary mounts, in the same
  order, so the "same routes as the server" claim is true again. The `doc-examples` module edit
  re-renders `docs/src/deployment/topologies` pages that `{{#include}}` it — the executor runs
  `scripts/check-doc-examples.sh` and `mdbook build docs/` after the edit (Phase 35 D-00b's
  re-run rule applied in the other direction) and records both green.
- **D-20:** **EX-121** (11 programs absent from `examples/README.md`): each gets a section in the
  existing house shape — `### [name.rs](name.rs)`, a **Demonstrates:** line, one sentence, the
  `cargo run --example … [--features …]` command, and a **Key concepts:** list — placed in a
  Table-of-Contents section that fits (new TOC sections **Vision**, **Document Processing**,
  **HTTP Service Host**, **RAG & Retrieval**, **Commander Strategies (Council / Grove /
  Conclave)** are added; `war_engine_memory_baseline.rs` joins **Performance Benchmarking**).
  The D-15 cluster programs each get a section the same way, under new TOC sections named for
  the cluster. The section headers stay parseable by the audit's own
  `grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]'` so a later audit can re-run the listed-vs-on-disk
  cross-check mechanically.
- **D-21:** **EX-122** (three README snippet lines read `response.content`,
  `response.token_usage.total_tokens`, `response.execution_time`): corrected to the real
  `PaladinResult` fields (`output`, `usage.total_tokens`, `execution_time_ms`). Going forward,
  **new README sections carry no "Code snippet" block** — the program file is the snippet;
  nothing checks README snippets for compile-currency (Layer 1b covers only the root README quick
  example), so bounded drift beats a fourth copy of every API shape. Existing snippet blocks are
  left in place once corrected.
- **D-22:** **EX-01:** the README's "Rust 1.70 or later" becomes **Rust 1.88** (the workspace
  `rust-version`, Phase 22.1 SS-08). While on that page, the "Run with Specific Features" block is
  checked against the root `Cargo.toml` feature list and any feature it names that does not exist
  is corrected — a currency fix on the same page, recorded under EX-01's closure, not a new ID.

#### Doc-test rule boundary (SC3 and the ROADMAP goal's "where the public-API rule applies")
- **D-23:** Phase 36 **does not** add `# Examples` headings to the 19 MISSING entry points, does
  not refreeze `16-DOCS-03-ENTRY-POINTS.md` at 101, and does not wire
  `scripts/check-public-api-examples.sh` into CI or `make`. All three are Phase 36.1 SC2's
  explicit deliverable, and Phase 34 D-00e ruled the frozen 76-item set is the rule's scope. What
  Phase 36 guarantees is narrower and mechanical: `cargo test --workspace --doc` stays green on
  every commit, every RD fix that rewrites a doc block keeps its existing doctest compiling, and
  every **new** example program is a real `examples/*.rs` binary, not a doctest. —
  **Reversibility:** reversible — 36.1 can still choose to fix the 19; nothing here forecloses it.

#### Closure bookkeeping (Phase 34/35 house pattern)
- **D-24:** Every plan's SUMMARY carries a **closure table**: `ID | file:line (cited) | file:line
  (actual, if different) | kind | fix | commit`. `RD-nn` followers appear on their lead's row.
  The phase-level VERIFICATION reproduces the full 143 + 64 (+ delta) map from those tables
  and the two bar commands' zero output.
- **D-25:** Verbatim command output lives in **`36-EVIDENCE.md`** (house `NN-EVIDENCE.md`
  pattern, Phase 34 D-02) with a `36-evidence/` subdirectory for the long captures (per-crate
  sweeps, example builds and runs). The CI run that proves D-12 is in **`36-CI-EVIDENCE.md`**
  (Phase 33 pattern).
- **D-26:** Commit granularity: **one commit per crate** for rustdoc fixes (subject
  `docs(36): resolve rustdoc links in paladin-<crate> (RD-nn…RD-mm)`), **one commit per
  program** for examples (`docs(36): add <name> example (EX-nn, EX-mm)`), one for the README,
  one for the gate wiring + closing measurement (D-13). Plain `git add -- <files> && git commit
  -m …` with a long timeout — the pre-commit hook runs workspace clippy whenever `.rs` or
  `Cargo.toml` is staged (Phase 35 D-24; project memory). Never `git add .`.
- **D-27:** `WINDOWS.md` rows **36 and 37** move to `fixed` through `gsd-tools` only
  (anti-pattern rule 15), in the closing plan, with the reason citing the closing commit and the
  zero-warning capture; row 37's reason names `RD-01`/`RD-66`/`RD-126` (the
  `HeuristicTokenCounter` link at `crates/paladin-memory/src/token_counter/mod.rs:3`). No new
  WINDOWS rows are added for findings this phase can close itself.
- **D-28:** `make api-surface` is run before every commit that touches `src/` or `crates/`
  and as the last gate; its "no change" output is recorded in `36-EVIDENCE.md`. If a fix
  genuinely cannot be made without a public change, the executor **stops** and records it as a
  deviation for the planner rather than editing `MIGRATION.md` §9.2 and the semver allowlist on
  its own — none is anticipated (D-05 forbids the one mechanism that would cause it).
- **D-29:** Security instructions apply to the new examples: the webhook-receiver example
  (EX-96/EX-97) verifies `X-Paladin-Signature` over the raw request bytes and never logs the
  secret; no example prints an API key, and examples that read keys from the environment say so
  in their header comment (`examples/README.md`'s `.env` convention).

### Claude's Discretion
- Exact plan count and wave shape. The audit's own suggestion: rustdoc by crate in parallel
  worktrees (battalion 72 rows and core 28 rows are the big ones; llm 13, web 11, facade 10,
  memory 3, storage 2, ports 2), then examples by cluster in parallel, then README, then the
  closing gate/evidence plan.
- Whether the RAG cluster (D-15) extends `paladin_with_rag.rs` or adds a sibling.
- How EX-107 (the CLI `eval run` capability) is made "runnable" (a program that spawns the CLI
  versus a checked-in scenario file with a README command).
- File names for the cluster programs, subject to the existing snake_case gallery convention
  and the Medieval-military vocabulary.
- Whether the per-crate `-D warnings` sweep is added to `make doc-check` as a third step or left
  as a research/verification tool (the workspace all-features run already fails on the first red
  crate, which is sufficient as a gate).

### Deferred Ideas (OUT OF SCOPE)
- **Regenerating `docs/src/appendix/doc-coverage-report.md`** from a real measurement once the
  bar is green — Phase 35 archived it with a banner pointing here; a later docs pass owns the
  regeneration.
- **Adding `make check-examples` (D-14) to the pre-push hook** — four full example builds are too
  slow for a push hook; revisit if the examples job ever goes red on a merge.
- **A `docs.rs`-style `#[cfg_attr(docsrs, doc(cfg(...)))]` feature-badge build** — out of scope;
  D-07 keeps links feature-consistent without it.
- **Aligning the `ci.yml` lint job's rustdoc steps with a `RUSTDOCFLAGS` matrix** (default vs
  all-features in one step) — D-12 adds a single explicit step instead; consolidation is a CI
  tidy-up for a later milestone.
- **Fixing `contributing-providers.md` lines 272/367 and the other unowned `docs/src` prose
  defects** — Phase 36.1 SC1.
- **The 19 MISSING `# Examples` headings, refreezing the entry-point file, wiring
  `check-public-api-examples.sh`** — Phase 36.1 SC2 (D-23).
- Not in this phase: `docs/src` prose (Phase 35, closed); the `ci.yml:538` example-count comment
  *except* where this phase's own edit to that step makes the count wrong anyway (D-15);
  PROJECT.md corrections; the two pending todos (coverage-reproduction verification, MinIO/RustFS
  evaluation) — both routed to Phase 36.1 SC5.
</user_constraints>

<phase_requirements>
## Phase Requirements

Phase requirement IDs are minted at planning time under the `CURR-*` prefix (D-00h); this
research maps ROADMAP Phase 36's five Success Criteria (SC1-SC5) to the research that supports
each, since inventing `CURR-nn` numbers here would conflict with the planner's numbering.

| Success Criterion | Description | Research Support |
|---|---|---|
| SC1 | `cargo doc --workspace --no-deps` zero `warning:` lines under the exact `ci.yml` command; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exits 0 | Re-baseline delta section below: both commands re-run verbatim, 65 default-feature warnings / 77 all-features errors confirmed byte-identical to `34-AUDIT.md` §3; fix techniques per warning kind (D-05..D-08) with worked examples and one newly-discovered technique gap (bare-module-name shorthand links) documented in Common Pitfalls |
| SC2 | Every Phase 34 RD-nn finding closed at cited crate/file/line; both rustdoc commands wired into `make clean-code`/pre-push so the count cannot regrow | Gate-wiring section: exact Makefile/`.pre-commit-config.yaml`/`ci.yml` edit points with line numbers, template hooks identified, real wall-clock timings measured this session |
| SC3 | `cargo build --examples` passes under all four CI feature-set splits; `cargo test --workspace --doc` green | Re-baseline delta: all four invocations re-run green this session (default 48/48, vision, content-processing, web-server); doctest totals re-confirmed 462/0/210 byte-identical to D-00f baseline |
| SC4 | Every Phase 34 EX-nn finding closed: obsolete examples updated/deleted with CHANGELOG note; every Phase 22-33 capability gap gets a runnable example in `examples/README.md` | Cluster-by-cluster API surface research (Architecture Patterns, Code Examples sections): confirmed import paths for every D-15 cluster's core types via `paladin::` facade re-exports or direct crate paths; `MockLlmAdapter` availability confirmed unconditional (root `[dependencies]`, not feature-gated) |
| SC5 | `make api-surface` reports no change; any genuine public-surface need goes through `MIGRATION.md` §9.2 + semver allowlist | Baseline `make api-surface` re-run clean this session (3959 items, 9.27s, "API surface unchanged") — confirms the starting point before any phase edits |
</phase_requirements>

## Summary

This phase closes a **read-only, already-fully-scoped** work list: `34-AUDIT.md` §6 names every
`RD-nn` (143) and `EX-nn` (64) row by file:line, and `36-CONTEXT.md` D-04..D-22 already specifies
the fix technique for each warning *kind* and the cluster grouping for every example gap. There
is no technology choice to make — the tooling is `cargo doc`, `rustdoc`'s own lints, `cargo test
--doc`, and the existing example/`doc-examples` compile-verification scripts, all already in the
tree. The research value this phase needs is therefore **empirical, not exploratory**: (1) prove
the re-baseline is still accurate (D-01/D-02's mandatory re-run), and (2) resolve the technique
questions the CONTEXT decisions leave open — which exact path fixes which unresolved link, which
crate/module paths the new example clusters import from, and where precisely the gate-wiring
edits land.

**Re-baseline result (the headline finding): zero drift.** Every quantitative figure in
`34-AUDIT.md` reproduces exactly on the current HEAD. The default-feature run still emits exactly
65 individual `warning:` lines (73 total `warning:`-prefixed output lines including the 8
per-crate summary lines) across the same 8 crates with the same per-crate counts. The per-crate
`-D warnings --all-features` sweep still finds exactly 77 content errors across the same 8
crates (`paladin-content`, `paladin-notifications`, `paladin-herald`, `paladin-eval`, and the new
`paladin-doc-examples` all still measure zero). The doctest baseline is still exactly 462
passed / 0 failed / 210 ignored. All four `ci.yml:548-558` example-build invocations still pass,
and `scripts/check-doc-examples.sh` still passes 0 checked / 622 skipped (docs snippet layer) / 0
failed. `make api-surface` is still clean (3959 items). Phase 35's six new `doc-examples` modules
introduced **zero** new rustdoc warnings (`paladin-doc-examples` sweeps 0 errors under both
default and all-features). **No `RD-144`+ or `EX-123`+ rows are needed; no §6 row is
closed-by-drift.** The planner can treat `34-AUDIT.md` §6 as current without adjustment.

One genuinely new technical finding surfaced during the fix-technique research (not in
`34-AUDIT.md`): a cluster of "unresolved link" warnings in module-level (`//!`) doc comments
that name **sibling `pub mod` names by their bare identifier** (e.g. `` [`bridges`] `` inside
`engine/mod.rs`'s own top-of-file module doc, referring to `pub mod bridges;` declared later in
the same file) fail to resolve even though the target is genuinely public and in the same
module's namespace. This affects the `engine/mod.rs` "Submodules:" list cluster (RD-13/14/15/16/
17/18/19/20/21/22/23/24/25 and their all-features followers, ~14 of the 143 rows). The reliable
fix, confirmed by contrast against a working example three lines above in the same file (a
`///` doc comment successfully linking via `` [`graph::WarGraph`] `` path syntax), is to use an
explicit path or the `mod@` disambiguator rather than the bare shorthand — see Common Pitfalls.

**Primary recommendation:** Plan this phase as `34-AUDIT.md` §6 prescribes — no new re-audit
wave is needed. Sequence: rustdoc fixes by crate in parallel worktrees (battalion and core are
the largest), then example clusters in parallel, then the README pass, then a single closing
wave that wires the gate (Makefile/pre-commit/ci.yml) and captures the final zero-warning
evidence in one commit (D-13). Use the exact fix techniques below for the two technique
questions CONTEXT leaves partially open: bare-module-name links (use `mod@` or a path) and
feature-gated cross-links (already fully enumerated below — 12 all-features-exclusive errors,
all explained by 4 known feature gates).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rustdoc link/HTML defect repair | Source (crate-owning tier: core/ports/battalion/llm/storage/web/facade) | Documentation/Tooling | Fixes are one-line doc-comment edits inside the crate that owns the defect; no cross-tier change |
| Doctest currency | Source (same as above) | Documentation/Tooling | `cargo test --doc` runs against the same doc comments the rustdoc fix touches; a rewording that breaks a doctest is a same-file regression |
| New example programs (WarEngine, control-flow, HITL, shutdown, node-cache, agent runtime, observability/eval, token economy, RAG) | Documentation/Tooling (`examples/`) | Backend/API (for the Platform API cluster, which starts the shipped `paladin-web` axum app in-process) | Each program is a standalone binary exercising the facade's public API surface; the Platform API cluster is the only one that stands up an HTTP server tier locally |
| Platform API client examples (EX-91..99, EX-104) | Backend/API (in-process axum host via `agent_router`/`thread_router`/`run_router`) | Documentation/Tooling | Reuses the shipped `paladin-web` router-assembly exactly as `src/bin/paladin-server.rs` does; a `reqwest` client in the same process drives it — no external service tier |
| Webhook signature example (EX-96/EX-97) | Backend/API (HMAC verification over raw bytes) | Security | Must follow the security-instructions credential-handling rules (D-29): verify over raw bytes, never log the secret |
| `examples/README.md` gallery | Documentation/Tooling | — | Pure documentation-tier artifact; no runtime component |
| Gate wiring (`make doc-check`, pre-push hook, CI lint-job step) | Build/CI tooling | — | Makefile, `.pre-commit-config.yaml`, `.github/workflows/ci.yml` — none of these are application code |
| `make api-surface` no-change guarantee | Build/CI tooling | Source | The gate itself lives in `scripts/check-api-surface.sh`; the guarantee it enforces is about the `src/`/`crates/` public surface (Source tier) |

## Standard Stack

This phase introduces **no new external dependency**. Every tool used is already in the tree and
pinned:

### Core (existing, no version change)
| Tool | Version (measured this session) | Purpose | Why Standard |
|------|------|---------|--------------|
| `rustdoc` (via `cargo doc`) | bundled with `rustc 1.97.1` | Generates docs, emits the lint warnings this phase closes | The tool the bar (`ADR-0033`) is defined against; no alternative documented-lint tool is in scope |
| `cargo test --doc` | `cargo 1.97.1` | Runs every doctest in the workspace | Already the SC3 gate; `--tests` and `llvm-cov` both skip doctests (project memory: "Doctests escape Paladin gates"), so this command is the only thing that exercises them |
| `pre-commit` (local hooks, `language: system`) | pinned per `.pre-commit-config.yaml` | Runs `cargo fmt`/`clippy`/build/test/doc-examples/api-surface at commit/push stages | Existing project convention; `doc-check` (D-11) is added as one more `language: system` entry, not a new tool |
| GitHub Actions `lint` / `examples` jobs | `.github/workflows/ci.yml` | CI gate that D-12 extends | Existing required checks |

### Supporting (already-pinned workspace crates, reused by new example programs)
| Crate | Version (from `Cargo.toml`) | Purpose | Which D-15 cluster uses it |
|-------|---------|---------|-----------------------------|
| `schemars` | `1.2` (workspace-pinned, already used by `paladin-battalion`/`paladin-eval`) | JSON Schema derivation for structured output | Agent runtime & middleware cluster (EX-88/EX-90) |
| `axum` | `0.8.4` (root `[dependencies]`, optional, `web-server` feature) | In-process HTTP host for the Platform API cluster | Platform API client cluster (EX-91..99, EX-104), reusing `examples/http_service_host.rs`'s pattern |
| `reqwest` | workspace-pinned (`blocking`, `stream` features already enabled on the facade) | HTTP client driving the in-process axum host | Platform API client cluster |
| `paladin-llm` `mock` feature (`MockLlmAdapter`, `MultiStepMockLlmPort`) | in-tree, unconditionally compiled into the facade (root `Cargo.toml:109`, `default-features = false, features = ["mock"]` is in `[dependencies]`, **not** `[dev-dependencies]`) | Offline LLM double for every new example (D-16) | All clusters that need an LLM call |
| `paladin-storage` `redis-cache` / `redis-queue` features | in-tree | Node-result cache / `RunQueuePort` Redis backend | Node-result cache cluster (EX-81), Platform API cluster (EX-98) — build-only in this environment (no Redis server running here) |
| `paladin-eval` crate + `eval_scenarios!` macro | in-tree (`crates/paladin-eval/src/runner.rs:682`) | Eval scenario authoring/running | Observability & eval cluster (EX-105..107) |

### Alternatives Considered
None — the phase's own CONTEXT.md already forecloses alternatives (D-11..D-17 specify the exact
Makefile targets, hook stages, and CI steps; no competing tool was evaluated because none is
needed).

**Installation:** None. No `cargo add`/`npm install` runs in this phase.

**Version verification:** `cargo --version` → `cargo 1.97.1 (c980f4866 2026-06-30)`; `rustc
--version` → `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Matches CONTEXT D-03's stated devcontainer
toolchain exactly.

## Package Legitimacy Audit

**Not applicable — this phase installs no new external package.** Every crate the D-15 example
clusters need (`schemars`, `axum`, `reqwest`, `paladin-eval`, `paladin-storage/redis-cache`,
`paladin-storage/redis-queue`) is already a pinned workspace dependency, verified by direct
`Cargo.toml` inspection this session (root `Cargo.toml` `[features]` block, `crates/paladin-eval/
Cargo.toml`, `crates/paladin-battalion/Cargo.toml`). No `npm view`/`pip index`/`cargo search`
lookup is needed because nothing new is added to any `Cargo.toml` `[dependencies]` table.

## Architecture Patterns

### System Architecture Diagram — the doc-check gate flow (SC1/SC2/SC3)

```
 developer edits a .rs doc comment or adds a new examples/*.rs program
                        |
                        v
        +-------------------------------+
        |  make doc-check (new, D-11)   |
        |  1. cargo doc --workspace     |
        |     --no-deps | tee | grep    |  <- fails if ANY "warning:" line (D-00a bar 1)
        |  2. RUSTDOCFLAGS="-D warnings"|
        |     cargo doc --workspace     |
        |     --all-features --no-deps  |  <- fails on first red crate (D-00a bar 2)
        |  3. cargo test --workspace    |
        |     --doc                     |  <- fails on any doctest regression (SC3)
        +-------------------------------+
                        |
          make clean-code (fmt, lint, lint-shell, check, doc-check)
                        |
          pre-push hook: doc-check entry (files: ^(src|crates)/.*\.rs$|^Cargo\.toml$)
                        |
                        v
          CI lint job: existing default-feature step (ci.yml:62-63)
                        + NEW all-features -D warnings step (D-12, after it)
                        |
          CI examples job: 4 feature-split cargo build --example invocations (unchanged)
                        |
                        v
          36-CI-EVIDENCE.md: real run recorded before phase verification (D-12)
```

### System Architecture Diagram — the Platform API example cluster (EX-91..99, EX-104)

```
  cargo run --example platform_api_client --features web-server
                        |
                        v
       in-process axum app assembly (mirrors src/bin/paladin-server.rs exactly):
       agent_router(state).merge(thread_router(thread_state)).merge(run_router(run_state))
                        |             (this is the EX-33/EX-55 fix target — the CURRENT
                        |              examples/http_service_host.rs stops at agent_router)
                        v
       axum::serve on a local ephemeral port (tokio::net::TcpListener)
                        |
                        v
       reqwest::Client in the SAME process drives, in order:
       POST /v1/runs (EX-91) -> GET /v1/runs/{id}/stream SSE (EX-92)
         -> POST /v1/runs/{id}/cancel (EX-93, optional path)
         -> /v1/assistants* (EX-94) -> /v1/schedules* (EX-95)
                        |
                        v
       webhook receiver (separate small axum handler, EX-96/EX-97):
       verifies X-Paladin-Signature HMAC over the RAW request body bytes
       it captured itself, never re-serialised (security.instructions.md rule)
```

### Recommended example-cluster file layout (new files only; existing 48 untouched)
```
examples/
├── war_engine_configuration.rs        # EX-62..66, EX-80 (WarEngine cluster; name at planner's discretion)
├── control_flow_dynamic_routing.rs    # EX-67..70
├── human_in_the_loop_gate.rs          # EX-71..73
├── graceful_shutdown.rs               # EX-74..76
├── platform_api_client.rs             # EX-77..79, EX-91..99, EX-104 (required-features = ["web-server"])
├── node_result_cache.rs               # EX-81..82 (required-features = ["redis-cache"]; build-only here)
├── agent_runtime_middleware.rs        # EX-83..90
├── observability_tracing.rs           # EX-100..104, EX-108 (EX-103 sub-path may need required-features = ["otel"])
├── eval_scenarios_demo.rs             # EX-105..107 (paladin-eval as a dev-dependency)
├── token_economy_commissary.rs        # EX-109..115
└── paladin_with_rag.rs (extend)       # EX-116..120 (D-15 discretion: extend vs sibling)
```
Exact names are Claude's Discretion (CONTEXT); the count above (10 new files + 1 extended) fits
the CONTEXT's own "≈10-14 programs" estimate for 59 gap rows.

### Pattern 1: Router-parity fix (EX-33/EX-55)
**What:** `examples/http_service_host.rs` and `crates/doc-examples/src/http_service_host.rs`
currently assemble only `agent_router(state).merge(docs_router(spec))`. The shipped binary
mounts three routers.
**When to use:** Any place a doc or example claims "the same app the server runs."
**Example (the real recipe, verified in `src/bin/paladin-server.rs:230-235`):**
```rust
// Source: src/bin/paladin-server.rs:228-235 (verified in-tree, this session)
// "thread_router's and run_router's output are merged ALONGSIDE agent_router's"
let routes = agent_router(state.clone())
    .merge(thread_router(thread_state))
    .merge(run_router(run_handles.run_state));
let app = if let Some(spec) = openapi_spec {
    routes.merge(paladin::infrastructure::web::openapi::docs_router(spec))
} else {
    routes
};
```

### Pattern 2: Offline-first example via `MockLlmAdapter`
**What:** Every new program that needs an LLM call uses the facade's already-unconditional
`MockLlmAdapter` rather than a real provider key.
**When to use:** All D-15 clusters except where the capability itself requires a real provider
(EX-111, Anthropic cache-token shape — demonstrated via the mock adapter's usage shape plus a
comment, per D-15's own text, since no live key runs in CI).
**Example:**
```rust
// Source: paladin::MockLlmAdapter re-export, src/lib.rs:192 (verified in-tree)
use paladin::MockLlmAdapter;
let llm = std::sync::Arc::new(MockLlmAdapter::new(/* canned response(s) */));
```

### Pattern 3: Bare-name intra-doc link fix for sibling `pub mod` references
**What:** A module-level (`//!`) doc block naming a sibling `pub mod` by bare shorthand
(`` [`bridges`] ``) fails to resolve even when the module is declared `pub mod bridges;` later
in the same file — empirically confirmed this session (see Common Pitfalls). The one line three
rows above that succeeds uses path syntax instead.
**Fix:** Use an explicit path or the `mod@` disambiguator, never the bare shorthand, for
module-to-module references inside a `//!` block:
```rust
// Working pattern (confirmed: no warning), crates/paladin-battalion/src/engine/mod.rs:59-61
/// a [`graph::WarGraph`] or [`graph_doc::WarGraphDoc`] as a Mermaid

// Failing pattern (confirmed: "no item named `bridges` in scope"), same file, line 20
//! - [`bridges`] — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`

// Recommended fix (either form resolves per rustdoc's own disambiguator syntax):
//! - [`bridges`](mod@bridges) — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
// or:
//! - [`self::bridges`] — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
```
This pattern applies to the whole "Submodules:" list cluster in `engine/mod.rs` (RD-13 through
RD-25 and their all-features followers RD-70 through RD-82 — 7 location groups, 14 rows) and to
the analogous cross-crate module-name links in the same warning family (`crate::engine::node`
references from `paladin-core`).

### Anti-Patterns to Avoid
- **Widening visibility to satisfy a link (`pub(crate)` → `pub`):** forbidden by D-05 — moves
  the `make api-surface` baseline and violates SC5.
- **`#[allow(rustdoc::private_intra_doc_links)]` or any other lint-suppression attribute:**
  forbidden by D-05 — hides the defect the bar exists to catch rather than fixing it.
- **`cargo check --example <name> --all-features` as a "does it build" proxy for examples:**
  the current `scripts/check-all-examples.sh` does exactly this and it is the wrong check — see
  Common Pitfalls; `--all-features` silently satisfies every `required-features` gate, which is
  precisely what CI's split into 4 invocations exists to catch (`ci.yml:538-546`'s own comment).
- **Assuming a `cargo doc --workspace --all-features` run enumerates every all-features error:**
  it does not — the workspace build aborts once enough crates fail (only 5 of the 13 crates
  reached "Documenting" before the run gave up in this session's re-run); the per-crate sweep
  (D-14 Phase 34 pattern) is the only complete enumeration.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Detecting rustdoc warnings | A custom regex/AST scanner over doc comments | `cargo doc` / `RUSTDOCFLAGS="-D warnings" cargo doc` | rustdoc's own lints (`private_intra_doc_links`, `broken_intra_doc_links`, `redundant_explicit_links`, `invalid_html_tags`) are the ground truth the CI gate checks; a custom scanner can drift from rustdoc's own resolution rules (as this session's module-name-link finding shows — even a human reading the source could not predict resolution failure without running rustdoc) |
| Verifying doctest currency | A script that greps doc comments for code fences | `cargo test --workspace --doc` | Already wired, already the SC3 gate, already distinguishes `ignore`/`no_run`/`compile_fail` doctest modes correctly |
| Checking every example builds under its real feature gate | A bespoke feature-matrix generator | The four `ci.yml:548-558` invocations, mirrored into the rewritten `scripts/check-all-examples.sh` (D-14) | CI's split already encodes the exact required-features partition; duplicating that logic elsewhere risks drift the moment a new gated example is added |
| Verifying the public API surface didn't move | Manual `cargo doc` diffing | `make api-surface` / `scripts/check-api-surface.sh` (already wired, uses `cargo-public-api`) | Already the SC5 gate; produced a clean, reproducible "3959 items, unchanged" result in 9.27s this session |
| HMAC signature verification for the webhook-receiver example (EX-96/EX-97) | A hand-rolled HMAC comparison | The pattern the shipped webhook delivery code already uses (`X-Paladin-Signature: sha256=<hex>` over raw bytes, `security.instructions.md`'s documented invariant) | Security-sensitive; the shipped code's own documented pattern (sign once over the exact byte buffer, never re-serialise) is the only correct approach — an example that reimplements this differently would teach the wrong pattern |
| JSON Schema derivation for structured output (EX-88/EX-90) | A manual schema builder | `schemars::schema_for!` (already workspace-pinned at `1.2`, already used by `paladin-battalion`/`paladin-eval`) | Same crate the shipped `StructuredExecutorPort` machinery already depends on; using anything else would demonstrate an API the framework doesn't actually use |

**Key insight:** every "don't hand-roll" in this phase is really the same insight restated: **the
gate commands and the shipped production code are the source of truth**; this phase's job is to
make the doc comments and examples agree with what already exists and already runs in CI, not to
invent new verification machinery.

## Common Pitfalls

### Pitfall 1: Bare shorthand links to sibling `pub mod` names silently fail to resolve
**What goes wrong:** A module-level `//!` doc comment lists its own child modules with bare
shorthand links (`` [`bridges`] ``), and rustdoc reports "no item named `bridges` in scope" even
though `pub mod bridges;` is declared later in the exact same file.
**Why it happens:** Empirically confirmed but not fully root-caused this session (flagged
`[ASSUMED]` below on the *mechanism*, not the *symptom* — the symptom is directly reproduced).
The nearby working counter-example (`` [`graph::WarGraph`] `` using path syntax, three lines away
in the same file, generates no warning) rules out "this file's doc comments never resolve" as an
explanation.
**How to avoid:** Use `mod@` disambiguation or an explicit `self::`/`crate::` path for every
module-to-module reference inside a `//!` block, never the bare shorthand — see Architecture
Patterns, Pattern 3.
**Warning signs:** `= note: no item named 'X' in scope` where `X` is a lowercase identifier that
matches a `pub mod` declared in the same or a sibling file — distinguish this from a genuinely
removed/renamed item (D-06's other category) by checking `grep -n "pub mod X"` in the same file
first.

### Pitfall 2: `cargo doc --workspace --all-features` under `-D warnings` is a floor, not a count
**What goes wrong:** Running the single workspace-wide all-features command and counting `error:`
lines undercounts the true defect list, because cargo aborts the whole workspace doc build once
enough dependency crates fail — in this session's re-run, only 5 of 13 crates reached
"Documenting" (`paladin-eval`, `paladin-content`, `paladin-herald`, `paladin-notifications`,
`paladin-web`) before 5 crates errored (`paladin-memory`, `paladin-web`, `paladin-battalion`,
`paladin-storage`, `paladin-ai`) and the run stopped; `paladin-llm`, `paladin-ai-core`,
`paladin-ports` never even started documenting in that single run.
**Why it happens:** `cargo doc`'s workspace build graph shares the same fail-fast semantics as
`cargo build` once `-D warnings` turns a lint into a hard error.
**How to avoid:** Always use the per-crate sweep (`RUSTDOCFLAGS="-D warnings" cargo doc -p
<crate> --all-features --no-deps` for all 13 crates) as the actual enumeration, exactly as
Phase 34 D-14 already mandates. The workspace-wide command is only useful as the SC1 pass/fail
gate, never as a defect inventory.
**Warning signs:** A workspace all-features run's `error:` count that is suspiciously close to
(but not exactly) the sum you'd expect — check `grep -n "Documenting" <output>` to see how many
crates actually started.

### Pitfall 3: `cargo check --example <name> --all-features` hides required-features gaps
**What goes wrong:** `scripts/check-all-examples.sh` (the current, pre-D-14 version) runs `cargo
check --example <name> --all-features` per file. Because `--all-features` enables every feature
including the ones a `[[example]]`'s `required-features` list names, this check can never
reproduce the failure CI's bulk `cargo build --examples` (no extra features) would hit — a
gated example that silently regresses to needing a feature not actually declared in
`required-features` would still pass this script while failing (or silently skipping) in CI.
**Why it happens:** The script predates the CI job's 4-invocation split (`ci.yml:538-546`'s own
comment documents that `cargo build --examples`'s bulk selector silently skips
`required-features` targets — verified live during Phase 4 research, cited in the CI comment).
**How to avoid:** D-14's rewrite — run the four `ci.yml:548-558` invocations verbatim instead of
a per-file `--all-features` loop.
**Warning signs:** `scripts/check-all-examples.sh` passes green but the CI "Example Muster" job's
binary-count assertion fails, or a `required-features` example silently stops needing the feature
it's declared to need without any check catching it.

### Pitfall 4: The `ci.yml:538` comment's example count is already stale, independent of this phase
**What goes wrong:** The CI comment states "examples/ holds 47 .rs files." This session's
`find examples -name '*.rs' | wc -l` returns **48**, and the binary-count assertion
(`EXPECTED=$(find examples -name '*.rs' | wc -l)`) already self-corrects at runtime — so CI does
not fail today, but the human-readable comment is wrong by one and has been since at least the
Phase 34 audit SHA (`git diff --stat` shows zero changes to `examples/` between the audit SHA and
current HEAD).
**Why it happens:** Pre-existing drift, not something this phase's own D-01 re-baseline
introduced — the comment was already wrong at audit time.
**How to avoid:** Since D-17 already requires editing this exact comment for every newly gated
example this phase adds, update the base "47" to the corrected total (48 existing + however many
new default-feature examples this phase adds) in the same edit, rather than treating the
off-by-one as a separate finding requiring its own row.
**Warning signs:** None needed going forward — the binary-count assertion is self-correcting;
this is purely a documentation-accuracy note for whoever edits that comment block.

### Pitfall 5: The default-feature run's "generated N warnings" lines are themselves `warning:`-prefixed
**What goes wrong:** `grep -c "^warning:"` on the raw `cargo doc --workspace --no-deps` output
returns 73, not 65, because the 8 per-crate summary lines (`` warning: `paladin-battalion` (lib
doc) generated 36 warnings ``) also start with `warning:`. The CI gate's own `grep -q "warning:"`
check doesn't care about the distinction (any match fails the gate), but a human or script trying
to *count* individual defects must subtract the summary lines.
**Why it happens:** rustdoc's own output format re-uses the `warning:` prefix for both individual
diagnostics and the per-crate rollup line.
**How to avoid:** Use `34-rustdoc-rows.sh` (the audit's own parser, explicitly reusable per
`36-CONTEXT.md`'s canonical refs) rather than a bare `grep -c` when an exact count matters.
**Warning signs:** A count that's off by exactly the number of crates with at least one warning.

## Code Examples

### `make doc-check` target (D-11) — drafted from the existing Makefile's style conventions
```makefile
# Source: pattern matches Makefile:328-331 (lint) and Makefile:377-380 (api-surface) style
.PHONY: doc-check
doc-check: ## Zero-warning rustdoc bar (default + all-features) plus doctests (ADR-0033, D-00a)
	@echo "$(CYAN)Checking documentation (default features, zero warnings)...$(NC)"
	@$(CARGO) doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt
	@! grep -q "warning:" /tmp/doc-output.txt
	@echo "$(CYAN)Checking documentation (all features, -D warnings)...$(NC)"
	@RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --all-features --no-deps
	@echo "$(CYAN)Running documentation tests...$(NC)"
	@$(CARGO) test --workspace --doc
```
Note `clean-code`'s existing dependency chain is `fmt lint lint-shell check` (Makefile:419); D-11
adds `doc-check` as a fifth prerequisite.

### `.pre-commit-config.yaml` new hook entry (D-11) — template is the adjacent `check-api-surface` entry
```yaml
# Source: pattern matches .pre-commit-config.yaml's existing check-api-surface entry (same file/feature filter)
      - id: doc-check
        name: rustdoc zero-warning bar + doctests (ADR-0033)
        entry: make doc-check
        language: system
        stages: [pre-push]
        pass_filenames: false
        files: ^(src|crates)/.*\.rs$|^Cargo\.toml$
```

### CI lint-job new step (D-12) — inserted directly after `ci.yml:62-63`
```yaml
# Source: pattern matches ci.yml:62-63's existing "Check documentation" step
      - name: Check documentation (all features, -D warnings)
        run: RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

### `[[example]]` declaration template for a new gated program (D-17)
```toml
# Source: Cargo.toml:453-456 (http_service_host's own entry, the closest analog for the
# Platform API cluster)
[[example]]
name = "platform_api_client"
path = "examples/platform_api_client.rs"
required-features = ["web-server"]
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| 73-warning baseline "carried, not a gate" (`33-CI-EVIDENCE.md` row 26) | Zero-warning bar enforced in the required `lint` CI job (`ci.yml:62-63`), default-feature side only | Already true before this phase (ADR-0033, ratified 2026-08-08) — the *default*-feature bar was already a hard gate; this phase's job is closing the residual 65 warnings that made even the default run non-zero at Phase 34's measurement, plus adding the *all-features* side as a second gate | Once closed, `cargo doc --workspace --no-deps` genuinely emits nothing — no more "pre-existing, unrelated" carve-outs like `WINDOWS.md` rows 36/37 |
| `scripts/check-all-examples.sh` runs `cargo check --example <name> --all-features` per file | (this phase, D-14) rewritten to run the four `ci.yml:548-558` invocations verbatim | This phase | Local check and CI check finally test the same thing; a `required-features` regression is caught locally, not just in CI |
| Examples-gallery README documents 37/48 programs (11 undocumented, per EX-121) | Every program gets a `### [name.rs](name.rs)` section (D-20) | This phase | `examples/README.md` becomes a complete, cross-checkable index (`grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]'` already used by the Phase 34 audit itself) |

**Deprecated/outdated:** None — this phase deprecates no API, only repairs documentation
artifacts.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The exact rustdoc *mechanism* causing bare-shorthand sibling-module links to fail to resolve (as opposed to the *symptom*, which is directly reproduced and empirically confirmed this session) is not fully diagnosed — the recommended fix (`mod@` disambiguator or explicit path) is inferred from rustdoc's own documented disambiguator syntax and confirmed by the working counter-example three lines away, not by reading rustdoc's resolver source | Common Pitfalls Pitfall 1, Architecture Patterns Pattern 3 | Low — even if the *mechanism* theory is imprecise, the *fix* is directly testable per-row during execution (`cargo doc -p paladin-battalion --no-deps 2>&1 \| grep -c warning`), so a wrong theory costs at most one extra edit-and-recheck cycle per row, never a wrong final state |
| A2 | The 10-11 new example filenames proposed under Architecture Patterns ("Recommended example-cluster file layout") are illustrative, not prescriptive — CONTEXT explicitly leaves file names to Claude's Discretion | Architecture Patterns | None — explicitly flagged as discretionary in CONTEXT itself; the planner is free to rename |
| A3 | EX-103's sub-capability (wiring the `otel` Cargo feature) may need its own `required-features = ["otel"]` example target separate from the rest of the observability cluster, rather than being folded into one `observability_tracing.rs` file — not confirmed empirically this session (would require attempting the build) | Architecture Patterns (file layout table) | Low — if wrong, the fix is splitting one file into two during execution, a same-wave adjustment, not a re-plan |

**If this table is empty:** N/A — three items above need no user confirmation before planning
(all are either self-correcting during execution or explicitly already discretionary per
CONTEXT); listed for completeness per the researcher template's requirement.

## Open Questions

1. **Exact new-example filenames and whether the RAG cluster extends `paladin_with_rag.rs` or
   adds a sibling**
   - What we know: CONTEXT explicitly defers both to Claude's Discretion; existing gallery
     naming convention is descriptive snake_case (`war_engine_memory_baseline.rs`,
     `battalion_checkpoint_recovery.rs`).
   - What's unclear: No further research narrows this — it is a planning-time choice, not a
     technical unknown.
   - Recommendation: Planner decides at plan-writing time; this research's proposed layout table
     is a reasonable default the planner can adopt or override.

2. **How EX-107 (CLI `eval run <glob>` capability) is made "runnable" as an example**
   - What we know: `paladin-cli`'s `eval run` subcommand exists behind the `cli` feature
     (`src/application/cli/commands/eval.rs`); `examples/*.rs` binaries conventionally don't spawn
     the workspace's own CLI binary as a subprocess.
   - What's unclear: Whether a program that shells out to `cargo run --bin paladin -- eval run
     <path>` via `std::process::Command` is idiomatic for this gallery, versus a README section
     with a checked-in scenario file and a documented command (no `.rs` file at all).
   - Recommendation: CONTEXT already frames this as Claude's Discretion; given every other
     `EX-nn` gap row maps to "exactly one named program" (D-15), a thin `.rs` wrapper that shells
     out (with clear stdout framing) is more consistent with the rest of the gallery than a
     README-only entry, but either satisfies the phase's own stated boundary.

3. **Whether the per-crate `-D warnings --all-features` sweep becomes a third `make doc-check`
   step**
   - What we know: CONTEXT explicitly lists this as Claude's Discretion; the workspace-wide
     all-features command (step 2) already fails on the first red crate, which is a sufficient
     pass/fail gate (Pitfall 2 above describes why it's an *insufficient enumeration* — but
     enumeration completeness is a *research/verification* need, not a *gate* need).
   - What's unclear: Nothing technical — purely a "is the extra ~10s of CI/local time worth the
     more precise failure attribution" tradeoff.
   - Recommendation: Leave it as a research/verification tool invoked manually (as this session
     did) rather than a third `make doc-check` step — the workspace all-features command is
     sufficient as a gate, and the per-crate sweep is mainly useful when actively debugging which
     crate broke, which is a rare event once the bar is green.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo`/`rustc` | All rustdoc/build/test commands | Yes | `cargo 1.97.1` / `rustc 1.97.1` | — |
| Redis server | EX-81 (`redis-cache`), EX-98 (`RunQueuePort` Redis backend) — build verification only | No (`redis-cli` binary present, no server running; `redis-cli ping` → connection refused) | — | Build-only per D-16: `cargo build --example <name> --features redis-cache` proves compilation; the program is not run in this environment, and CI's examples job does not run it either (build-only, per D-16's own text) |
| Docker | Not required by this phase's own gates | No (`docker` not on PATH in this shell) | — | Not needed — no `test-integration-docker` target is part of this phase's validation surface |
| OTLP collector endpoint | EX-102/EX-103 (`otel` feature) — build verification only | No | — | Build-only per D-16, same pattern as Redis above |
| GitHub Actions runner (for D-12's CI-evidence capture) | D-12's mandatory real-CI-run evidence | Not available from this shell — requires pushing the branch | — | Executor must push and capture a real run per D-12/D-25 (`36-CI-EVIDENCE.md`); cannot be simulated locally |

**Missing dependencies with no fallback:**
- None — every missing dependency above has an explicit, CONTEXT-sanctioned fallback (build-only
  verification, or a required real-CI-run step that is inherently a "push and capture" action
  rather than a local-shell dependency).

**Missing dependencies with fallback:**
- Redis, OTLP collector, Docker — all build-only fallback per D-16; no test or example run in
  this phase's validation depends on them being live.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Cargo's built-in `cargo test --doc` (rustdoc-driven doctest harness) plus `cargo doc` itself as a lint-check "test" |
| Config file | None dedicated — behavior is driven by `#![warn(missing_docs)]` crate attributes (already in every `lib.rs`) and `RUSTDOCFLAGS` env var at invocation time |
| Quick run command | `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt` — measured this session at **0.92s** warm-cache (cargo's own incremental doc cache), **~27s** cold |
| Full suite command | `make doc-check` (D-11): the above, plus `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` (measured this session: fails fast on first red crate; full per-crate enumeration sweep across all 13 crates took well under a minute combined, individual crates ranging from 0.78s to a few seconds each, warm-cache), plus `cargo test --workspace --doc` (measured this session: **~20s** total across all `Doc-tests` sub-runs, warm-cache) |

### Phase Requirements → Test Map
| Success Criterion | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SC1 | Zero rustdoc warnings, default features | lint/static | `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt` | Yes (`ci.yml:63`) |
| SC1 | Zero rustdoc errors, all features, `-D warnings` | lint/static | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | New — D-12 adds to CI; `make doc-check` (D-11) adds locally |
| SC1 (enumeration/debug aid) | Per-crate all-features error count | lint/static | `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` (13 crates) | Existing pattern, Phase 34 D-14; not itself a new gate |
| SC2 | Gate wired so count cannot regrow | integration (tooling) | `make clean-code` (includes `doc-check`); pre-push hook `doc-check` entry | New — Makefile + `.pre-commit-config.yaml` edits |
| SC3 | `cargo build --examples` under all 4 feature splits | build/smoke | `cargo build --examples --offline`; `cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline`; `cargo build --example document_processing --features "content-processing" --offline`; `cargo build --example http_service_host --features "web-server" --offline` | Yes (`ci.yml:548-558`); all 4 re-confirmed green this session |
| SC3 | `cargo test --workspace --doc` green | doctest | `cargo test --workspace --doc` | Yes (`ci.yml:499`); re-confirmed 462/0/210 this session |
| SC3 | Doc-examples crate + README snippet + docs/src snippets compile-verified | build/smoke | `./scripts/check-doc-examples.sh` | Yes; re-confirmed green this session (0 checked/622 skipped/0 failed for the docs/src layer, which is expected — no docs/src edits this phase; the doc-examples crate and README-snippet layers did check and pass) |
| SC4 | Every EX-nn gap has a runnable program | smoke (manual run + exit-code capture) | `cargo run --example <name>` per new program, offline where D-16 allows | New — one run per new program, recorded in `36-EVIDENCE.md` |
| SC4 | New gated examples build under their feature | build/smoke | `cargo build --example <name> --features <feature>` per new gated target, added to both `ci.yml` and the rewritten `scripts/check-all-examples.sh` | New |
| SC5 | Public API surface unchanged | static diff | `make api-surface` (`./scripts/check-api-surface.sh .project/current-exports.txt`) | Yes; re-confirmed clean this session (3959 items, 9.27s) |

### Sampling Rate
- **Per plan/wave commit:** `cargo doc -p <crate> --no-deps 2>&1 \| grep -c 'warning:'` and
  `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` for the crate just
  touched (D-10's per-crate verification-as-you-go pattern); `cargo run --example <name>` for any
  example program just added/edited.
- **Per wave merge:** `make doc-check` (all three steps) plus `make api-surface`.
- **Phase gate (last wave, D-13):** the full `make doc-check`, the rewritten
  `scripts/check-all-examples.sh` (all four CI invocations), `scripts/check-doc-examples.sh`,
  `make api-surface`, and a real pushed-branch CI run captured in `36-CI-EVIDENCE.md` before
  `/gsd-verify-work`.

### Wave 0 Gaps
None — every test command this phase needs already exists and was proven runnable this session
(`cargo doc`, `cargo test --doc`, the four example-build invocations, `check-doc-examples.sh`,
`check-api-surface.sh`). The only *new* automation is the gate-wiring itself (`make doc-check`,
the pre-push hook entry, the CI step), which is Deliverable work for this phase (D-11/D-12), not
a pre-requisite test-infrastructure gap.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | This phase adds no authentication surface; the Platform API example cluster reuses the shipped server's existing auth wiring unchanged |
| V3 Session Management | No | No session-management code is added |
| V4 Access Control | No | No access-control code is added |
| V5 Input Validation | Partial — example-only | The webhook-receiver example (EX-96/EX-97) validates an inbound signature header; no new input-validation library is introduced — it reuses the shipped pattern |
| V6 Cryptography | Yes — HMAC verification in the webhook-receiver example | `X-Paladin-Signature: sha256=<hex>` verified over the raw captured request body bytes, matching `security.instructions.md`'s documented invariant exactly; **never hand-rolled** — the example demonstrates the same HMAC-SHA256 comparison the shipped `WebhookDeliveryService` already performs, using the same crate (no new crypto dependency) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Example prints an API key read from the environment | Information Disclosure | D-29: examples that read keys from the environment say so in their header comment (the existing `.env` convention); no example prints the value itself — an audit grep for `println!.*_API_KEY` (or similar) across new example files is a cheap verification step during review |
| Webhook-receiver example logs the shared secret or the raw signature-verification input in a way that could leak it | Information Disclosure | D-29: never logs the secret; verifies over raw bytes captured once, matching the shipped `webhook_deliveries` row's own "no signing key on the row" prohibition (P1, documented in `crates/paladin-core/src/platform/container/webhook.rs:10-15`) |
| A doc-comment fix that de-links a private item accidentally exposes internal implementation detail in prose (e.g. naming an internal cache key algorithm) | Information Disclosure (low severity — docs, not code) | D-05's "reword to name the public entry point" guidance keeps prose focused on the public contract, not internal mechanics; reviewer should sanity-check that a de-linked mention doesn't leak more detail than the private item's own doc comment already would via `--document-private-items` (which nobody runs against the public docs site) |
| A new example accidentally becomes runnable in CI against a real external service (Redis/OTLP) without the service being available, causing flaky CI | Denial of Service (of CI, not the app) | D-16: services-requiring programs are **build-only** in CI (`cargo build --example`, never `cargo run --example`); this is already the pattern for every existing example needing external services (confirmed: no existing example is executed in `ci.yml`'s examples job, only built) |

## Re-baseline delta

**Method:** every command D-01 specifies was re-run verbatim on HEAD `619551a781f6e352fac896d5fab1ba8630876585`
(toolchain `cargo 1.97.1` / `rustc 1.97.1`) in this research session, with output captured under
`36-evidence/` (scratchpad; the executor should re-capture into the phase's own `36-evidence/`
directory per D-25 rather than relying on this session's temp files). Read-only throughout — no
tracked file was modified; `cargo doc`/`cargo build`/`cargo test` wrote only to `target/`.

**Pre-check:** `git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- src crates
examples Cargo.toml ':!crates/doc-examples'` → **empty output** (zero changes outside
`crates/doc-examples`), confirming CONTEXT D-01's claim. `git diff --stat
ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- crates/doc-examples` → the same 8-file, 557-
insertion diff CONTEXT already describes (six new modules + `lib.rs`/`Cargo.toml` registration).

### 1. `ci.yml:63` command (default features)
```
$ cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt
[... 65 individual warning: diagnostics, 8 per-crate "generated N warnings" summary lines ...]
CI-COMMAND-EXIT:1
```
Per-crate summary lines (byte-identical to `34-AUDIT.md` §3's "Default-feature enumeration"):
`paladin-ai` 5, `paladin-web` 3, `paladin-storage` 1, `paladin-battalion` 36, `paladin-memory` 1,
`paladin-llm` 4, `paladin-ports` 1, `paladin-ai-core` 14. **Sum: 65** (matches audit exactly).
Total `warning:`-prefixed lines: 73 (65 diagnostics + 8 summary lines — see Pitfall 5).

### 2. Workspace `-D warnings --all-features` command
```
$ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
EXIT:101
```
Confirms bar 2 still fails (expected — 77 content errors remain, per §3 below). Only 5 of 13
crates reached "Documenting" before the run aborted (`paladin-eval`, `paladin-content`,
`paladin-herald`, `paladin-notifications`, `paladin-web` started; `paladin-memory`,
`paladin-web`, `paladin-battalion`, `paladin-storage`, `paladin-ai` errored; `paladin-llm`,
`paladin-ai-core`, `paladin-ports` never started) — confirms Pitfall 2's "floor, not
enumeration" finding, matching `34-AUDIT.md`'s own framing of this exact command.

### 3. Per-crate `-D warnings --all-features` sweep (all 13 crates)
| Crate | Exit | Content errors (this session) | `34-AUDIT.md` §3 figure | Match? |
|---|---|---|---|---|
| `paladin-ai-core` | 101 | 14 | 14 | Yes |
| `paladin-ports` | 101 | 1 | 1 | Yes |
| `paladin-battalion` | 101 | 36 | 36 | Yes |
| `paladin-llm` | 101 | 9 | 9 | Yes |
| `paladin-memory` | 101 | 1 | 1 | Yes |
| `paladin-storage` | 101 | 1 | 1 | Yes |
| `paladin-content` | 0 | 0 | 0 | Yes |
| `paladin-notifications` | 0 | 0 | 0 | Yes |
| `paladin-web` | 101 | 8 | 8 | Yes |
| `paladin-herald` | 0 | 0 | 0 | Yes |
| `paladin-eval` | 0 | 0 | 0 | Yes |
| `paladin-ai` (facade) | 101 | 7 | 7 | Yes |
| `paladin-doc-examples` | 0 | 0 | (not in original 34-AUDIT.md scope — new crate at re-measure time) | **New crate, zero warnings** |

**Sum: 77** (14+1+36+9+1+1+8+7), matching CONTEXT D-07's stated "77 errors against the default
run's 65" exactly. `paladin-doc-examples` (Phase 35's six new modules) is the one crate not
covered by the original `34-AUDIT.md` per-crate table because it did not carry Phase 35's modules
at audit time — it measures **zero** errors under both default and all-features this session,
confirming Phase 35 introduced no new rustdoc defect.

**Feature-gated delta (all-features-only, 12 = 77 − 65):** every one of the 12 errors that exist
only under `--all-features` traces to a known, unconditional feature gate — no mystery items:
- `src/application/cli/commands/eval.rs:281` (paladin-ai facade) — gated by the `cli` feature (1 error)
- `src/infrastructure/telemetry/otel_sink.rs:42` (paladin-ai facade) — gated by the `otel` feature (1 error)
- `crates/paladin-web/src/dev_ui_controller.rs` (5 locations: lines 3, 20, 28, 69, 131) — gated by the `dev-ui` feature (5 errors)
- `crates/paladin-llm/src/compat/engine.rs` (3 locations: lines 114, 201, 1041) — gated by the `openai-compatible` feature (3 errors)
- `crates/paladin-llm/src/gemini/adapter.rs` (2 locations: lines 28, 59) — gated by the `gemini` feature (2 errors)

This directly satisfies D-07's requirement to identify which link targets are feature-gated: the
doc'd items themselves are the ones behind the feature gate in every one of these 12 cases (not
just the link *target*), so D-07's "a doc comment may link to a feature-gated item only when the
doc'd item is gated by the same feature" rule is trivially satisfiable — fix each in place with
the same D-05/D-06 technique, verified to hold under both builds by re-running the per-crate
sweep after the fix (D-10).

### 4. `cargo test --workspace --doc`
```
test result: ok. 462 passed; 0 failed; 210 ignored; 0 measured; 0 filtered out
```
Byte-identical to `34-AUDIT.md`'s D-00f baseline (462/0/210). **Zero drift.**

### 5. The four `ci.yml:548-558` example invocations + `scripts/check-doc-examples.sh`
| Invocation | Result |
|---|---|
| `cargo build --examples --offline` (default, 44 auto-discovered targets — see note) | EXIT 0 |
| `cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline` | EXIT 0 |
| `cargo build --example document_processing --features "content-processing" --offline` | EXIT 0 |
| `cargo build --example http_service_host --features "web-server" --offline` | EXIT 0 |
| Binary-count assertion (`ci.yml:565-576` logic, re-run manually) | `Expected: 48; found: 48` — **matches**, confirming Pitfall 4's "the comment says 47, the tree has 48" finding is pre-existing, not new drift |
| `scripts/check-doc-examples.sh` | EXIT 0 — `paladin-doc-examples` crate compiles, README Quick Example in sync, docs/src snippet layer: 0 checked / 622 skipped / 0 failed (expected — no `docs/src` edits this phase) |

`--offline` worked without modification — the devcontainer's registry cache is warm; no need to
drop the flag per D-01's contingency clause.

### Reconciliation against `34-AUDIT.md` §6
- **Rows that no longer reproduce (`closed-by-drift`):** **none.**
- **New warnings/errors with no §6 row (`RD-144`+ / `EX-123`+):** **none.**
- **Rows confirmed to still reproduce exactly as audited:** all 143 `RD-nn` rows (via the
  identical per-crate warning/error counts above) and all 64 `EX-nn` work-item rows (via the
  identical example-build results and the unchanged `git diff --stat` against `examples/`,
  which the currency/gap findings depend on — no capability newly shipped or removed between the
  audit SHA and current HEAD outside `crates/doc-examples`, which is itself confirmed
  warning-free).

**Conclusion: the re-baseline is a clean pass-through.** `34-AUDIT.md` §6 remains valid without
modification. The planner should treat it as current and does not need to re-run this
measurement again before planning (though the executor should re-capture fresh evidence at
execution time per the house pattern, since evidence capture and planning are different
concerns).

## Sources

### Primary (HIGH confidence — re-measured this session against the live tree)
- `cargo doc --workspace --no-deps` (this session) — 65 warnings, 8 crates, byte-identical to audit
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` (this session) — exit 101, partial enumeration confirming Pitfall 2
- `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` × 13 crates (this session) — 77 total errors, byte-identical to audit
- `cargo test --workspace --doc` (this session) — 462/0/210, byte-identical to audit
- Four `ci.yml:548-558` example-build invocations (this session) — all green, 48/48 binaries
- `scripts/check-doc-examples.sh` (this session) — green
- `./scripts/check-api-surface.sh .project/current-exports.txt` (this session) — clean, 3959 items, 9.27s
- `src/bin/paladin-server.rs:228-235` — the exact router-merge recipe for the EX-33/EX-55 fix
- `crates/paladin-battalion/src/engine/mod.rs:1-90` — the bare-shorthand-link vs. path-syntax contrast that grounds Pitfall 1/Pattern 3
- `crates/paladin-core/src/platform/container/webhook.rs:1-40, 175-190` — confirms `WebhookDelivery`/`WEBHOOK_DELIVERY_SCHEMA_VERSION` genuinely exist and are unconditional
- Root `Cargo.toml:104-115` (`[dependencies]` block) — confirms `MockLlmAdapter`'s `mock` feature is unconditional, not a `[dev-dependencies]`-only convenience
- Root `Cargo.toml:487-554` (`[features]` block) — confirms `redis-cache`, `otel`, `dev-ui`, `web-server`, `gemini`, `openai-compatible`, `cli` feature names
- `src/lib.rs:178-227`, `src/prelude.rs:1-58` — confirms facade re-export paths for `Commissary`, `ShedItem`, `WindowSource`/`WindowFallbackPolicy`/`ResolvedWindow`/`resolve_context_window`, `MockLlmAdapter`, `ExecutionMiddleware`, `ConfinedVault`

### Secondary (MEDIUM confidence — read from canonical phase documents, not independently re-verified beyond citation)
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §3, §4, §6 — the full RD-nn/EX-nn work list and Kind classification (re-verified in aggregate via the per-crate counts above, not row-by-row)
- `.planning/phases/34-documentation-currency-audit/34-CONTEXT.md` — D-23 re-run rule text
- `.planning/phases/35-mdbook-currency/deferred-items.md` — confirms no pending anchor-change pointer inherited by Phase 36
- `.planning/decisions/0033-cargo-doc-warning-bar.md` — the ratified bar and the `doc-examples` `missing_docs` disposition
- `.planning/WINDOWS.md` rows 36/37 — the two ledger rows this phase closes
- `CHANGELOG.md` `[0.10.0]` `### Documentation` (line 421) — the append point per D-00j
- `.pre-commit-config.yaml`, `Makefile` — the hook/target templates this phase extends

### Tertiary (LOW confidence — theory, not independently verified against rustdoc's own resolver behavior)
- The exact internal rustdoc mechanism causing bare-shorthand module-name links to fail (Assumption A1) — the symptom and the fix are both empirically confirmed; the *why* is inferred from rustdoc's documented disambiguator syntax, not from reading rustdoc's source.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new tooling; every command verified runnable this session
- Architecture (RD fix techniques, gate wiring, example cluster structure): HIGH for the parts
  CONTEXT already specifies (D-04..D-22, directly copied and cross-checked against the tree);
  MEDIUM for the specific new-file layout proposal (illustrative, Claude's Discretion per
  CONTEXT)
- Pitfalls: HIGH for Pitfalls 2-5 (directly reproduced this session); MEDIUM for Pitfall 1's root
  cause (symptom HIGH-confidence, mechanism theory LOW-confidence per Assumption A1)
- Re-baseline delta: HIGH — every figure independently re-derived from fresh command output in
  this session, not copied from `34-AUDIT.md`

**Research date:** 2026-09-17
**Valid until:** This re-baseline is valid until the next commit touching `src/`, `crates/`
(excluding further additive `doc-examples` modules that don't introduce new doc links), or
`examples/` — i.e., it should be treated as current for the planning session that follows
directly from this research, but the executor must still re-verify per-crate warning counts as
each fix lands (D-10), since the whole point of the fixes is to change these exact numbers to
zero.
