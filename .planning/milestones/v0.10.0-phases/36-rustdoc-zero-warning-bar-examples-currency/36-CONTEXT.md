# Phase 36: Rustdoc Zero-Warning Bar & Examples Currency - Context

**Gathered:** 2026-09-17
**Status:** Ready for planning
**Mode:** `--auto` — every decision below is the recommended option, selected without user
prompts; the alternatives considered are in `36-DISCUSSION-LOG.md`.

<domain>
## Phase Boundary

Phase 36 delivers **the rustdoc corpus at the bar CI already enforces, and an examples gallery
that demonstrates the tree that ships.** Concretely:

- `cargo doc --workspace --no-deps` emits **zero** `warning:` lines under the exact `ci.yml`
  lint-job command (`ci.yml:63`), and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --all-features --no-deps` exits 0 — closing `WINDOWS.md` rows 36 and 37.
- Every one of the **143 `RD-nn` rows** and **64 `EX-nn` work rows** in `34-AUDIT.md` §6 is
  closed at its cited crate / file / line (the 58 `EX-nn` rows the audit confirmed `current`
  need no action). Closing a lead row's source line closes every ID in its `Blocks` cell.
- Both rustdoc commands and `cargo test --workspace --doc` are wired into the local gate set
  (`make clean-code` + pre-push) so the count cannot silently regrow, and the all-features run
  joins the CI lint job beside the default-feature command already there.
- `cargo build --examples` passes under each of the four feature-set invocations CI splits on
  (`ci.yml:548-558`), `cargo test --workspace --doc` is green, and every Phase 22-33 capability
  the audit's gap list flags has a runnable program under `examples/` listed in
  `examples/README.md`.
- `make api-surface` reports no change — every fix is documentation-side; a private-item link is
  never "fixed" by widening visibility.

**Not in this phase:** `docs/src` prose (Phase 35, closed; its unowned leftovers are Phase 36.1
SC1); the 19 MISSING `# Examples` headings and the `check-public-api-examples.sh` scope drift
(Phase 36.1 SC2); the `ci.yml:538` example-count comment *except* where this phase's own edit to
that step makes the count wrong anyway (D-15); PROJECT.md corrections; the two pending todos
(Phase 36.1 SC5); any behavioural change to library code.

</domain>

<decisions>
## Implementation Decisions

### Inherited — locked by earlier phases, not re-litigated
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

### Re-baseline before fixing — the D-23 rule is triggered
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
  `closed-by-drift` with the command output that proves it, never silently skipped. — **Reversibility:**
  costly — Phase 36.1 SC4 and Phase 37 verify by ID against this phase's closure table; a row
  that is neither closed nor dispositioned breaks that chain.
- **D-03:** Toolchain: the devcontainer carries cargo 1.97.1; the CI lint job's toolchain is what
  `ci.yml` pins. If the local zero and the CI count disagree on the closing commit, the CI figure
  wins (it is the gate) and the difference is recorded in `36-CI-EVIDENCE.md`, not explained
  away.

### Rustdoc fix technique — one rule per warning kind (SC1, SC2, SC5)
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

### Gate wiring — where the commands live so the count cannot regrow (SC2, SC3)
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

### Example gap list — 59 rows, ≈10-14 programs (SC4)
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

### Stale existing examples and the README (EX-01, EX-33, EX-55, EX-121, EX-122)
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

### Doc-test rule boundary (SC3 and the ROADMAP goal's "where the public-API rule applies")
- **D-23:** Phase 36 **does not** add `# Examples` headings to the 19 MISSING entry points, does
  not refreeze `16-DOCS-03-ENTRY-POINTS.md` at 101, and does not wire
  `scripts/check-public-api-examples.sh` into CI or `make`. All three are Phase 36.1 SC2's
  explicit deliverable, and Phase 34 D-00e ruled the frozen 76-item set is the rule's scope. What
  Phase 36 guarantees is narrower and mechanical: `cargo test --workspace --doc` stays green on
  every commit, every RD fix that rewrites a doc block keeps its existing doctest compiling, and
  every **new** example program is a real `examples/*.rs` binary, not a doctest. — **Reversibility:**
  reversible — 36.1 can still choose to fix the 19; nothing here forecloses it.

### Closure bookkeeping (Phase 34/35 house pattern)
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The work list and its evidence (the contract this phase closes)
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §6 — the 143 `RD-nn` and 64
  `EX-nn` work rows with Order, Size, Location, Cites, Blocks, Evidence anchor; the 58 `EX-nn`
  confirmed-current IDs; the reconciliation rules.
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §3 — per-row **Kind** and the
  verbatim first line of every rustdoc message (the §6 table omits Kind; §3 has it), the doctest
  baseline, the entry-point gate subsection.
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §4 — examples build table, the
  D-17(c) capability gap list, the `examples/README.md` audit, the D-18 module→page include map.
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` and
  `.planning/phases/34-documentation-currency-audit/34-evidence/` — raw captures the rows cite
  (`34-07-percrate/<crate>.txt`, `34-08-examples-builds.txt`, `34-07-doctests.txt`).
- `.planning/phases/34-documentation-currency-audit/34-CONTEXT.md` — D-00a…D-23 (bar, ID
  stability, sizing, measurement protocol, partition, D-23 re-run rule).
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` — what this phase must
  **not** absorb (entry-point script drift, `ci.yml:538` comment except per D-17, PROJECT.md).
- `.planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh` — the audit's own
  warning-to-row parser; reuse for the D-01 re-measure diff.

### Phase 35 boundary and house patterns
- `.planning/phases/35-mdbook-currency/35-CONTEXT.md` — D-11 (snippet rules), D-24 (commit
  granularity), D-25 (CHANGELOG Documentation subsection), D-26 (doc-examples boundary), D-27.
- `.planning/phases/35-mdbook-currency/deferred-items.md` — confirms no pending anchor-change
  pointer for Phase 36; lists the prose defects Phase 36.1 owns.
- `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` row 26 — the 73-warning
  carried baseline and the "carried, not a gate" precedent this phase ends.

### Decision records and ledgers
- `.planning/decisions/0033-cargo-doc-warning-bar.md` — the ratified bar; the `doc-examples`
  `missing_docs` disposition; the `#![warn(missing_docs)]` uniformity.
- `.planning/WINDOWS.md` rows 36 and 37 — the two ledger rows this phase flips to `fixed`.
- `.planning/ROADMAP.md` Phase 36 (SC1-SC5) and Phase 36.1 (SC2, SC4, SC5 — what is *not* ours).
- `MIGRATION.md` §9.2 — the rename register for unresolved links to renamed items (D-06).
- `CHANGELOG.md` `[0.10.0]` `### Documentation` — where Phase 36's bullets append (D-00j).

### Gates, hooks and CI (what gets edited in the last wave)
- `.github/workflows/ci.yml:59-63` — lint job; line 63 is the verbatim default-feature bar.
- `.github/workflows/ci.yml:499` — the existing `cargo test --workspace --doc` step.
- `.github/workflows/ci.yml:501-576` — "Example Muster" job: the four invocations, the 47-count
  comment, the binary-count assertion.
- `.pre-commit-config.yaml` — pre-push stage hooks (`check-api-surface` is the template for
  the new `doc-check` entry).
- `Makefile` — `test-doc` (:122), `check-doc-examples` (:163), `lint` (:329), `check` (:347),
  `api-surface` (:378), `doc` (:414), `clean-code` (:419).
- `scripts/check-all-examples.sh` — rewritten per D-14. `scripts/check-doc-examples.sh` —
  Layer 1/1b/2, run after any `doc-examples` edit. `scripts/check-api-surface.sh` — SC5.
- `Cargo.toml:444-462` — the four `[[example]]` declarations with `required-features`.

### Examples tree
- `examples/README.md` — the gallery page (EX-01, EX-121, EX-122); section convention
  `### [name.rs](name.rs)` / Demonstrates / command / Key concepts.
- `examples/http_service_host.rs`, `crates/doc-examples/src/http_service_host.rs` — EX-33/EX-55.
- `examples/war_engine_memory_baseline.rs`, `examples/maneuver_*.rs`,
  `examples/paladin_with_rag.rs`, `examples/battalion_checkpoint_recovery.rs` — nearest existing
  analogs for the engine, control-flow, RAG and checkpoint clusters.
- `crates/doc-examples/src/lib.rs`, `crates/doc-examples/src/support.rs`,
  `crates/doc-examples/Cargo.toml` — mock adapters and feature set available to compile-verified
  snippets; `superstep_engine.rs` (Phase 35) already builds a small cyclic `WarGraph`.
- `docs/src/user-guides/superstep-engine.md` — Phase 35's engine guide; the vocabulary and the
  `EngineLimits` / `APP_ENGINE_*` names the engine cluster examples must match.

### Project rules
- `CLAUDE.md` and `.github/copilot-instructions.md` — vocabulary table, working agreements,
  api-surface rule, no `unwrap()`/`expect()` in library code (examples may use `?` with
  `anyhow`/`Box<dyn Error>` main).
- `.github/instructions/security.instructions.md` — credential handling and webhook signature
  rules that the Platform API and webhook examples must respect (D-29).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`MockLlmAdapter`** (`paladin-llm`, `mock` feature) — offline LLM for every new example;
  already wired into `crates/doc-examples/Cargo.toml` (`features = ["openai", "mock"]`).
- **`crates/doc-examples/src/support.rs`** — shared mock adapters for compile-verified snippets;
  Phase 36 may edit it (D-00e) if a README-included snippet needs a new helper.
- **`crates/doc-examples/src/superstep_engine.rs`** (Phase 35) — builds a cyclic `WarGraph`,
  sets `EngineLimits`, runs and reads Waypoint history: the seed for the engine-cluster program.
- **`examples/http_service_host.rs`** — in-process axum host pattern; the Platform API client
  cluster reuses it and adds `reqwest` calls against the bound port.
- **`.planning/phases/34-documentation-currency-audit/34-rustdoc-rows.sh`** — parses `cargo doc`
  output into rows; the re-measure diff (D-01) should reuse it rather than re-derive.
- **`scripts/check-doc-examples.sh`** and the pre-push hook block in `.pre-commit-config.yaml` —
  the template for the new `doc-check` target and hook.

### Established Patterns
- **Zero-warning posture**: all eleven library crates and the facade carry `#![warn(missing_docs)]`;
  `doc-examples` deliberately does not (ADR-0033 amendment). Link hygiene is the whole gap.
- **Bulk `--examples` silently skips `required-features` targets** — every gated example needs
  its own CI and script invocation (`ci.yml:538-546`, verified live in Phase 4 research).
- **Evidence-first closure**: `NN-EVIDENCE.md` + `NN-evidence/` captures, closure tables in
  SUMMARYs, IDs cited in commit subjects, WINDOWS rows moved only via `gsd-tools`.
- **Hooks**: the pre-commit stage runs workspace clippy on any staged `.rs`; the pre-push stage
  runs build, lib tests, doc-examples, doc-config and api-surface. Executors commit with plain
  git and a long timeout.
- **Toolchain split**: devcontainer cargo 1.97.1 vs CI's pinned toolchain; count differences
  are findings, not noise (Phase 34 D-12).

### Integration Points
- `Cargo.toml` `[[example]]` block (lines 444-462) — new gated examples declare here.
- `.github/workflows/ci.yml` lint job (line 63) and Example Muster job (lines 548-576) — the
  all-features doc step and any new gated-example invocation land here.
- `Makefile` `clean-code` chain and `.pre-commit-config.yaml` pre-push stage — `doc-check`.
- `examples/README.md` TOC and sections — every new program and the 11 unlisted ones.
- `CHANGELOG.md` `[0.10.0]` `### Documentation` — appended bullets.
- `.planning/WINDOWS.md` rows 36/37 — flipped in the closing plan.

</code_context>

<specifics>
## Specific Ideas

- The engine-cluster example should read like the Phase 35 superstep-engine guide in code: build
  a small cyclic `WarGraph`, set `EngineLimits`, run, then print the Waypoint history and the
  `GRAPH_FINGERPRINT_VERSION` so the reader sees what a fingerprint bump would invalidate.
- The Platform API client example should print each route it calls and the status it got, in
  the order a real client would: submit run → stream → (cancel) → assistants → schedules, then
  the webhook receiver verifies a signature over raw bytes it captured itself.
- Every new program's header comment states: what it demonstrates (naming the capability the
  README section names), the command to run it, and whether it needs a key or a service.
- `make doc-check` output should read as three labelled steps so a red step is attributable at a
  glance (the Makefile's existing `$(CYAN)…$(NC)` echo style).

</specifics>

<deferred>
## Deferred Ideas

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

### Reviewed Todos (not folded)
- **Verify local `make coverage` reproduces CI's 82.39% figure**
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6) —
  matched on generic keywords (make, coverage, docs, workflows). Its documentation slice closed in
  Phases 34/35; the remainder is a Docker-machine walk that ROADMAP Phase 36.1 SC5 now owns
  explicitly. Not folded, for the same reason Phases 32-35 gave; the `--auto` ≥ 0.4 fold rule was
  deliberately set aside to avoid two owners for one item.
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.6) —
  matched on "yml, github, workflows, crates" only; an infrastructure decision with no rustdoc
  or example to correct, owned by Phase 36.1 SC5. Not folded.

</deferred>

---

*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Context gathered: 2026-09-17*
