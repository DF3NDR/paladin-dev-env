# Phase 33: Commissary In-Tree Adoption - Research

**Researched:** 2026-09-16
**Domain:** Rust workspace refactor — wiring an existing, fully-built allocator (`Commissary`)
into an existing consumer (`RagRetrievalService`) across a new lateral crate edge, plus a
release-gate re-seal
**Confidence:** HIGH (nearly everything below was read directly off the live tree on 2026-09-16,
the same day CONTEXT.md's own anchors were checked; no new external package beyond an
already-workspace-pinned dev-dependency)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Crate seam — where the `dispense` call lives (COMM-01)**
- **D-01:** The `Commissary::dispense` call lives **inside `RagRetrievalService`** in
  `paladin-memory`, exactly where the success criterion places it. To reach it,
  `crates/paladin-memory/Cargo.toml` gains `paladin-llm = { version = "0.10.0", path =
  "../paladin-llm", default-features = false }` — a lateral adapter→adapter edge with direct
  house precedent (`crates/paladin-battalion/Cargo.toml:67-69` depends on `paladin-llm` the
  same way, with its "No cycle: paladin-llm depends only on core + ports" comment;
  `paladin-content` takes it optionally). No cycle: `paladin-llm` depends on `paladin-core` and
  `paladin-ports` only. `default-features = false` keeps `reqwest`/`rand` out of the memory
  crate — `commissary` and `window` are unconditional modules in `paladin-llm`, so a
  featureless build is enough (the `crate-isolation` CI job's `cargo build -p paladin-memory
  --no-default-features` must stay green). The publish order needs no change:
  `scripts/publish-crates.sh`'s `CRATES` array (line 144) already lists `paladin-llm` before
  `paladin-memory`, and `cargo publish --workspace --dry-run` orders itself.
  — **Reversibility:** costly — `paladin-memory`'s public API will expose `paladin_llm`
  types (`ShedItem`, `CommissaryError`); undoing that later is another §9.2 break for the
  downstream consumer.
- **D-02:** Rejected alternatives, recorded so the planner does not reopen them: (a) moving the
  rationing into the facade application layer would leave `paladin-memory`'s own
  `retrieve_context` either unbounded or silently truncating — the success criterion names
  `RagRetrievalService` itself; (b) moving `Commissary` into `paladin-ports`/`paladin-core` was
  already rejected by Phase 32 (resolver locked in `paladin-llm`) and would be a second
  §9.2 break with no caller asking for it.
- **D-03:** Docs that draw the crate graph follow the edge in the same commit:
  `docs/src/architecture/crate-map.md` mermaid block (~lines 27-60) gains `mem --> llm`, and
  the `paladin-memory` section (~line 166) plus `crates/paladin-memory/src/lib.rs`'s crate
  narrative name the dependency and why (Commissary rationing).

**Building a Commissary against a bare `rag.max_tokens` budget (COMM-01)**
- **D-04:** RAG has no provider window, only an injection cap, so the service builds its
  Commissary with **`Commissary::new` over synthetic capabilities**: provider label `"rag"`,
  `ProviderCapabilities { max_context_tokens: Some(budget), ..Default::default() }`, and a
  `CommissaryPlan` with `reserved_completion_tokens: 0` (the whole budget is for memories)
  and `fallback_context_tokens: None`. **No new `Commissary` constructor** — `Commissary`'s
  public surface is unchanged this phase (Phase 32 PRD §4 stance carried forward). The
  `"rag"` label is what any surfaced `CommissaryError` names as its "provider", which is
  honest: the window being enforced is the RAG cap, not an LLM's.
- **D-05:** `rag.max_tokens` is `usize`; `Commissary` takes `u32`. Convert with
  `u32::try_from` and return a typed error on overflow — **never clamp** (ADR-0004 stance).
  `RagConfig::validate` already rejects `max_tokens == 0`, which is the only value that
  would trip `Commissary::new`'s `ReservationExceedsWindow` guard with `reserved = 0`.
- **D-06:** The Commissary is **constructed per call inside `retrieve_context`** (it holds an
  `Arc` clone, a small capabilities struct and a plan — negligible), so
  `RagRetrievalService::new` stays infallible and keeps its `(sanctum, embedding, config)`
  signature. Construction errors flow out through the new error type (D-12).
- **D-07:** The service needs a `TokenCounterPort`. `new` **defaults to
  `HeuristicTokenCounter`** (same crate, `crates/paladin-memory/src/token_counter/heuristic.rs`,
  `is_exact() == false`) and gains a **`with_token_counter(Arc<dyn TokenCounterPort>)`**
  builder — the exact shape `PaladinExecutionService::with_token_counter` already uses
  (Phase 26 D-13), so a facade user who injects `TiktokenCounter` there can inject it here
  too. The facade does not auto-propagate its own counter into an `Arc<RagRetrievalService>`
  it did not construct (deferred idea).

**Priority derivation from relevance score (COMM-01)**
- **D-08:** `ConsignmentItem.priority` is **rank order**: after `rank_by_relevance` (already
  sorted by `score` descending, stable), item `i` gets `priority = u8::try_from(i)
  .unwrap_or(u8::MAX)`. Lower number = higher priority = shed last, which is exactly the
  Commissary contract, and it makes "the highest-scoring memories are the ones retained" a
  structural guarantee rather than a rounding property (a scaled `(1 - score) * 255` would
  collapse close scores into ties and lose the ordering). `RagConfig::validate` caps `top_k`
  at 100, so the `u8::MAX` saturation is defensive only. Ties in `score` keep insertion
  order (the sort is stable) — documented in the rustdoc.
- **D-09:** `ConsignmentItem.label` is the **memory UUID** (`entry.memory.id.to_string()`),
  never the content. `ShedItem.label` therefore surfaces in results and logs without
  carrying memory text (security instructions: response bodies and stored content never
  reach logs un-redacted).

**Dispensing semantics for RAG (COMM-01, COMM-02)**
- **D-10:** `Commissary::dispense` is used with its **default semantics, unchanged**: equal
  byte share per retained item (`budget / n`, clamped to the plan's per-item bounds), the
  lowest-priority item shed until the provisional total fits, and any retained body longer
  than its share cut at a char boundary with the plan's `truncation_marker`. PRD R1 says
  "return the `Stockpile`", and that is the Stockpile's contract. Consequences the planner
  must carry into tests and docs: (a) a single memory larger than the whole budget is
  **retained truncated with the per-item marker**, not shed (today it is silently dropped
  and the prompt gets nothing — the new behaviour is strictly more informative);
  (b) a long high-scoring memory beside short ones can be cut to its share even when the
  total would fit — accepted; a "whole-item-only" mode on `CommissaryPlan` is a deferred
  idea, not this phase's scope. — **Reversibility:** reversible — plan values and a private
  seam; changing them later is a behavioural note, not an API break.
- **D-11:** Plan values: `pessimistic_tokens_per_1000_bytes` stays the Commissary default
  (`358`), `per_item_min_bytes`/`per_item_max_bytes` stay at defaults, `truncation_marker`
  stays `"\n... (truncated)"`, `model_hint` is empty (RAG does not know the completion
  model). `fixed` is the **empty string** — the "## Relevant Context" header is formatting,
  not budgeted, matching today's accounting. The pessimistic ratio means the bytes planned
  for injection are ~30 % fewer than the old `len / 4` estimate allowed at the same
  `max_tokens`; the `Stockpile`'s `prompt_tokens`/`allotted_tokens`/`exact_tally` are
  carried on the result (D-12) so a caller can see the real usage, and the CHANGELOG
  behavioural note states the change plainly (D-21).

**Result shape and error type — the COMM-02 surface**
- **D-12:** `retrieve_context` returns a **new result struct** (exact name is Claude's
  discretion; the shape is locked): a `Vec` of retained memories, each carrying the original
  `SanctumSearchResult` (entry + score), the **post-dispense `body`** (possibly cut and
  marker-suffixed — renderers must print this, never `memory.content`), and `truncated: bool`;
  `shed: Vec<ShedItem>` (the `paladin_llm` type, re-exported from `paladin-memory`'s
  services module and the facade's `sanctum` module so callers name one path); and the
  Stockpile accounting `prompt_tokens: u32`, `allotted_tokens: u32`, `exact_tally: bool`.
  Convenience accessors: `len()`, `is_empty()`, and a `was_rationed()`-style predicate
  (`!shed.is_empty() || any truncated`). `retrieve_context_with_timeout` returns the same
  struct (an empty one on timeout, as today). `format_for_prompt` takes the struct.
  — **Reversibility:** one-way — a published signature in `paladin-memory` and its facade
  re-export; the downstream app migrates once under ADR-0051.
- **D-13:** A **new `paladin-memory` error enum** for the RAG path (name discretionary):
  `Sanctum(#[from] SanctumError)`, `Commissary(#[from] CommissaryError)`, and the D-05
  budget-conversion variant — layer-specific `thiserror` enums converted at boundaries, the
  house pattern. `SanctumError` in `paladin-ports` is **not** extended (no port change). The
  facade's `retrieve_context_with_timeout`
  (`src/application/services/paladin/paladin_execution_service.rs:1923-1962`) keeps mapping
  into `PaladinError::ExecutionError` / `PaladinError::Timeout(5)` exactly as it does now.
- **D-14:** The rationing step is factored into a **synchronous seam** (private or
  `pub(crate)`, e.g. `fn ration(&self, ranked: Vec<SanctumSearchResult>) -> Result<…, …>`)
  called by `retrieve_context` after `rank_by_relevance`. The property test (D-17) drives
  that seam directly — no async runtime inside `proptest` closures.

**Marker emission and observability (COMM-02)**
- **D-15:** The RAG-level truncation marker is a **single trailing line in the rendered
  prompt**, emitted by **both** renderers when `shed` is non-empty:
  `RagRetrievalService::format_for_prompt` and the facade's `format_retrieved_context`
  (`paladin_execution_service.rs:1969-1985`). Wording is discretionary but must state the
  count of omitted memories and the budget (e.g. `[3 lower-relevance memories omitted to fit
  the 2000-token RAG budget]`) and come from **one shared helper/constant in `paladin-memory`**
  so tests and the facade never re-type the string. Commissary's own per-item marker stays
  inside cut bodies (D-10) — two markers with two meanings, both visible.
- **D-16:** Observability is **one `log::info!` line per retrieval** naming retained count,
  shed count, `prompt_tokens`/`allotted_tokens` and `exact_tally` — counts and ids only, never
  content. The facade's existing "RAG retrieval succeeded" `info!` gains the shed count. No
  `TraceEvent`, no herald field (deferred idea — PRD R2's "so callers/observability can see
  it" is satisfied by the result struct plus the log line).

**Test strategy and the COMM-03 exit grep**
- **D-17:** **Property test** with `proptest` (already pinned at `1.4` in the facade
  `Cargo.toml:246`; add `proptest = "1.4"` to `paladin-memory`'s `[dev-dependencies]`) in
  `rag_retrieval_service.rs`'s `#[cfg(test)]` module over the D-14 seam with
  `HeuristicTokenCounter`: for random vectors of (content, score) and budgets, assert
  (i) `prompt_tokens <= max_tokens`, (ii) every shed memory's score is ≤ every retained
  memory's score (rank-order shedding from the tail), (iii) retained ∪ shed = input by id
  (nothing lost). Use distinct, non-round fixture numbers in the example-based tests
  (Phase 31 house style).
- **D-18:** **Integration test** — a new, **ungated** facade test
  `tests/integration/rag_commissary_test.rs` registered in `tests/integration/mod.rs` next to
  `in_memory_sanctum_tests` (line 55; the existing `rag_integration_tests` is `qdrant`-gated
  at line 78 and needs Docker, which this devcontainer lacks). It goes through the facade
  re-export path (`paladin::application::services::sanctum::{RagRetrievalService, RagConfig}`)
  over `InMemorySanctum` with a deterministic mock `EmbeddingPort` (copy the pattern from
  `rag_integration_tests.rs:~200-255`), stores memories of distinct sizes, and asserts:
  small budget → `shed` non-empty AND marker present in `format_for_prompt`; large budget →
  `shed` empty AND no marker; one oversized memory → `truncated == true`, per-item marker
  present, `shed` empty (the D-10(a) edge). This is the F4 "production caller exercised by
  integration tests" evidence; it runs in CI's `integration-tests` job without services.
- **D-19:** **Exit grep (COMM-03):** `grep -rn 'truncate_to_token_budget' crates src docs
  examples benches` → empty, and `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` →
  empty (`CHANGELOG.md`, `MIGRATION.md`, `.planning/`, `.project/` are history and out of
  scope). The scout's sweep of every other `truncat*` site is recorded in the closing
  SUMMARY as "marked, not silent": Conclave's `truncate_output`
  (`crates/paladin-battalion/src/conclave_execution_service.rs:453-462`) appends
  `... [truncated]`; `content_summarizer_service.rs` is a character-length summariser by
  design, not a token budget; `redact.rs`/`trace.rs`/`node_error.rs` are redact-then-bound
  error-text paths with explicit markers. `commissary.rs:22-25`'s module doc, which names
  `truncate_to_token_budget` as the live anti-pattern, is rewritten in the past tense to name
  RAG as the first production caller (ADR-0049 is history and stays as written).
- **D-20:** The Phase 32 fold of PRD R3 (`HistoryTrimmer` on the shared resolver) leaves this
  phase **only the regression check**: `cargo test -p paladin-ai limit_resolution
  kept_set_equivalence_snapshot_pre_resolver` green, recorded in the last plan's SUMMARY. No
  code.

**Release bookkeeping and the COMM-04 re-seal**
- **D-21:** `CHANGELOG.md` `[0.10.0]` (line 18): a **`### Behavioral changes`** bullet — RAG
  output now carries a truncation marker and a shed record where it previously dropped
  silently, an oversized top memory is retained truncated rather than dropped, and the
  injected volume at a given `rag.max_tokens` is planned at the Commissary's pessimistic
  ratio (D-11); a **`### Changed`** bullet for `retrieve_context`'s result struct, the new
  error enum, `format_for_prompt`'s parameter and `with_token_counter`; and a **`### Added`**
  line for the `paladin-memory` → `paladin-llm` dependency. The Phase 31/32 entries are
  **verified present by grep**, not rewritten.
- **D-22:** `MIGRATION.md` §9.2 gets **one row per `crate | Type` pair** (Phase 29 D-04 keying):
  `paladin-memory | RagRetrievalService` (return-type and parameter breaks on
  `retrieve_context`/`format_for_prompt`, migration guidance: read `.memories`/`.shed`), and
  `paladin-memory | retrieve_context_with_timeout` for the free function's return type; a
  `paladin-ai` row only if a lint fires for the facade re-export. Lint discovery is
  **empirical** (Phase 32 D-14, `cargo-semver-checks@0.50.0`, `--release-type minor` on every
  local run, `--default-features` for `paladin-memory` and `paladin-ai`, plus
  `--features content-processing` for both since Phase 32 showed the gated surface differs):
  every lint that fires gets its `.cargo/semver-checks-allowlist.toml` `[[entry]]` in the
  **same commit**; a pair with zero fired lints keeps its §9.2 row as migration guidance
  marked `N/A` with the tool-coverage note (the Phase 32 `paladin-llm | Commissary` row,
  `MIGRATION.md:205`, is the template). `make check-migration-allowlist` must exit 0.
- **D-23:** The public-surface baseline is regenerated with `make api-surface-update` and
  `.project/current-exports.txt` lands **in the same commit** as the API change (the pre-push
  gate from quick task 260916-h40 rejects drift; today's baseline lists the RAG re-exports at
  lines 3962-3973).
- **D-24:** The re-seal is the **last plan of the phase**, run on the phase's final commit.
  Gates, each with its exact command (all Phase 29 / 32 precedents): `grep -c TBD
  MIGRATION.md` = 0; `make check-migration-allowlist` (and `make check-gates`);
  `cargo test --features web-server --test v0_9_config_boot`;
  `cargo test -p paladin-web --test openapi_golden_v0_9`; the CI `semver` job's per-package
  command (`ci.yml:~357`) for every publishable crate plus the D-22 discovery runs;
  `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked`;
  `make publish-dry-run` (`cargo publish --workspace --dry-run` — the CI `publish-dry-run` job
  runs only on `main` pushes, `ci.yml:1974-1978`, so the local run is the pre-merge evidence
  and the post-merge run is recorded afterwards per Phase 29 D-21's two-SHA rule);
  `make api-surface`; `make clean-code`; `make security`; the 82 % coverage floor (CI's exact
  `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path
  lcov.info --fail-under-lines 82 -- --test-threads=1`, or attributed to the CI `coverage` job
  when the devcontainer cannot run it); CHANGELOG grep for the RAG note + Phase 31/32
  entries. The pre-existing 72-warning `cargo doc --workspace --no-deps` condition is
  **recorded, not a gate** (Phase 29 §8, Phase 32 32-05 precedent).
- **D-25:** Evidence placement: a **new `## 11. Re-seal after Phases 30-33 (Phase 33,
  COMM-04)`** section appended to `.project/v0.10.0/09-program-acceptance-audit.md` (the
  corpus audit — Phase 29 D-10), one subsection per gate with command, verdict, head SHA and
  date; a phase-local **`33-CI-EVIDENCE.md`** in the same shape as `29-CI-EVIDENCE.md` (local
  sweep table + CI-run table); and a one-paragraph "Re-sealed on <SHA>, 2026-09-xx" note
  appended to `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md`'s pointer.
  **No new audit document.** Findings are recorded, never fixed silently (Phase 29 D-12);
  the seven maintainer sign-off boxes stay untouched and §11 adds **one more unticked
  human-only box** ("the `0.10.0` tag may be cut") — never ticked by an agent (Phase 29 D-17).
- **D-26:** Commit mechanics carried forward: `git add -- <files> && git commit -q -m …`
  directly (the GSD commit helper times out under the pre-commit workspace-clippy hook and
  reports `nothing_to_commit` for new files — Phase 30/32 `<specifics>`, memory notes);
  conventional scopes `build(33)` / `feat(33)` / `test(33)` / `docs(33)`; write "v0.10.0" in
  every new doc and row, never "v0.11.0" (Phase 30 D-18).

### Claude's Discretion
- Exact names of the result struct, its retained-memory item type, the error enum and the
  sync rationing seam (D-12/D-13/D-14); whether `ShedItem` is re-exported by `pub use` at
  `paladin_memory::services` only or also at the crate root.
- Exact wording of the RAG-level omission marker and of the `info!` line (D-15/D-16), subject
  to the count-and-budget rule.
- Whether the per-call Commissary construction (D-06) is wrapped in a tiny private
  `fn commissary(&self, budget: u32) -> Result<Commissary, …>` helper.
- Plan/wave granularity — natural waves: (1) `build(33)` dependency edge + docs graph +
  `proptest` dev-dep; (2) result type, error enum, sync seam, `dispense` adoption, both
  renderers and markers, unit + property tests (COMM-01/02); (3) the ungated integration
  test, exit grep, commissary module-doc rewrite, mdBook "in-tree caller" section, config
  docs (COMM-03); (4) §9.2 rows + allowlist + CHANGELOG + API baseline + the full gate
  re-seal with evidence in the corpus audit (COMM-04).
- Whether to also extend the `qdrant`-gated `rag_integration_tests.rs` with a marker
  assertion (harmless; the ungated test is the required evidence).

### Deferred Ideas (OUT OF SCOPE)
- A **whole-item-only dispensing mode** on `CommissaryPlan` (never cut a retained body;
  shed whole items instead) — a Commissary behaviour knob and a `constructible_struct_adds_field`
  break; backlog, only if a caller needs it.
- A **per-model context-window override table** for `Commissary` (Phase 32 deferred; still
  deferred).
- **Trace/herald surfacing of shed memories** (`TraceEvent` variant or `ExecutionMetadata`
  field) — new observability capability; Phase 28 owns the trace enum.
- The facade **auto-propagating its `TokenCounterPort`** into a caller-built
  `Arc<RagRetrievalService>` — would need the facade to construct the service; separate
  design.
- Migrating **Conclave's `truncate_output`** (already marked, char-based) onto the Commissary
  as a second caller — candidate, not required by COMM-03.
- Tuning `pessimistic_tokens_per_1000_bytes` per counter exactness (e.g. `250` when
  `is_exact()`), once real usage numbers exist.
- Treasurer, pricing, `cost_estimate`, allowances, pacing — Milestone 14 (ADR-0050).
- **Not in this phase (phase boundary):** changing RAG retrieval, scoring, filtering or
  deduplication; any change to `Commissary`'s dispensing algorithm or its public surface;
  wiring `HistoryTrimmer` onto the shared resolver (done in Phase 32); Treasurer/pricing;
  renaming any token type; migrating other already-marked truncation sites.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COMM-01 | RAG truncation goes through `Commissary::dispense` over a `Consignment` built from retrieved memories, priority from relevance score, budget `rag.max_tokens`; property test proves retained ≤ budget and highest-scoring retained | Verified `Commissary::dispense`/`Consignment`/`ConsignmentItem` API shape (below), `RagConfig`/`RagRetrievalService` current code, `HeuristicTokenCounter`, and the `paladin-battalion` dependency-edge precedent (with an important correction — see Pitfall 1) |
| COMM-02 | `ShedItem` list surfaced through the RAG result path; truncation marker emitted when content was shed; tests assert both directions | Verified `ShedItem`/`Stockpile` field shapes and their `Clone`/`Debug`-only derive set (Pitfall 2), the two renderer call sites in the facade, and the exact result-struct consumers that must migrate |
| COMM-03 | Integration test exercises `Commissary::dispense` through the real RAG path; no silent token-based truncation remains, grep-provable | Verified `tests/integration/mod.rs` gating (`in_memory_sanctum_tests` ungated at line 55, `rag_integration_tests`/`qdrant_sanctum_tests` both `#[cfg(feature = "qdrant")]` at lines 73/75), ran the D-19 exit-grep sweep now (baseline captured below), confirmed Conclave/content-summarizer/redaction sites are not silent |
| COMM-04 | Phase 29 gates re-sealed on the final commit: MIGRATION.md, `v0_9_config_boot`, OpenAPI golden diff, semver-checks, MSRV, `cargo publish --dry-run`, CHANGELOG `[0.10.0]` | Verified every named gate's exact command and current state locally: `cargo-semver-checks 0.50.0` and `cargo-llvm-cov 0.8.7` are installed, rustup has both `1.88` and `1.97.1` toolchains, Docker is **not** available (coverage evidence must be CI-attributed), and found a CHANGELOG discrepancy the planner must resolve (Pitfall 4) |

</phase_requirements>

## Summary

This phase is a pure in-tree wiring exercise, not new technology: `Commissary::dispense`
(`crates/paladin-llm/src/services/commissary.rs`) is fully built, fully tested (18 unit tests) and
already re-exported from the facade — it has simply never been called by production code. Every
type CONTEXT.md names (`Commissary`, `CommissaryPlan`, `Consignment`, `ConsignmentItem`,
`DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryError`) was read directly off the live file
and its shapes, defaults and error variants are documented exactly below. The RAG side
(`crates/paladin-memory/src/services/rag_retrieval_service.rs`) is a 253-line service with an
obvious, isolated removal target: `truncate_to_token_budget` (lines 187-212) and its one call site
(line 116) and its `len() / 4` estimate (line 196) are the entire surface being replaced.

No new external dependency is introduced beyond `proptest = "1.4"`, which is already a workspace
crate's dev-dependency (root `Cargo.toml:246`, used by no other crate in the workspace yet) — this
is a same-workspace, already-vetted crate, not a new supply-chain surface. The only genuinely new
*structural* thing is the `paladin-memory` → `paladin-llm` production dependency edge, and one
important correction to CONTEXT.md's own framing surfaced during verification: the cited
precedent (`paladin-battalion` → `paladin-llm`) is a **dev-dependency**, not a production one (see
Pitfall 1). The edge itself is still sound — `paladin-llm` truly depends on nothing but
`paladin-core`+`paladin-ports`, confirmed by reading its `Cargo.toml` and `lib.rs` in full — but
the planner should not describe it to a reviewer as "the same kind of edge that already exists in
production," because it is not; it is the first one.

The bulk of the remaining work is release bookkeeping (COMM-04), which is entirely mechanical and
was dry-run-testable in this session: `cargo-semver-checks@0.50.0` and `cargo-llvm-cov@0.8.7` are
both installed locally, `rustup` has both the `1.88` MSRV toolchain and the `1.97.1` default
installed, and Docker is confirmed absent (so the coverage gate's local evidence must be
CI-attributed, exactly as Phase 29/32 already did). `mdbook`/`mdbook-linkcheck` ARE installed
locally (contrary to CONTEXT.md's hedge), so the docs build can be checked locally too.

**Primary recommendation:** Wave order should mirror CONTEXT.md's own Claude's Discretion
suggestion exactly — (1) dependency edge + docs graph + `proptest` dev-dep, verified by the
`crate-isolation`-equivalent local build; (2) result type + error enum + sync `ration` seam +
`dispense` adoption + both renderers + unit/property tests; (3) integration test + exit grep +
module-doc rewrite + docs; (4) the full release re-seal. Do not attempt to shortcut wave 1 by
also touching `rag_retrieval_service.rs` in the same commit as the `Cargo.toml` edge — the
`crate-isolation` CI job builds `paladin-memory` twice (default features, then
`--no-default-features`) and a broken intermediate state on either build is easiest to isolate
when the dependency-edge commit contains no application code yet.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Token-budget rationing algorithm (priority sort, shed-until-fits, char-safe truncation) | Adapter (`paladin-llm::services::commissary`) | — | Already built and owned by `paladin-llm`; this phase adds no algorithm, only a caller |
| RAG retrieval orchestration (embed → search → filter → dedupe → rank → **ration**) | Adapter (`paladin-memory::services::rag_retrieval_service`) | — | `RagRetrievalService` is itself a port-only adapter service; the new `ration` step is inserted into its existing pipeline, same tier |
| RAG budget policy (`rag.max_tokens`, priority-from-score mapping) | Adapter (`paladin-memory`, caller-supplied `ConsignmentItem.priority`) | — | Commissary's own doc is explicit: "the Commissary never decides WHICH material matters more — that is the caller-supplied priority"; RAG owns this policy, not `paladin-llm` |
| Result surfacing (`shed`, `truncated`, marker) to the reasoning loop | Application / Facade (`PaladinExecutionService`) | Adapter (`paladin-memory`'s own `format_for_prompt`) | Both renderers must emit the marker (D-15) — the facade renderer lives in the application layer, the crate's own renderer is adapter-tier; they are two call sites of the same responsibility, not two different capabilities |
| Release-gate re-seal (MIGRATION.md, semver, coverage, publish dry-run) | Program/CI tooling | — | Not a runtime tier at all — this is release engineering, verified via `Makefile`/`ci.yml`/scripts, out of the Ports & Adapters model entirely |

## Standard Stack

No new external (non-workspace) dependency is introduced by this phase.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `proptest` | `1.4` (pinned, root `Cargo.toml:246`) | Property-based testing of the D-14 rationing seam | Already the workspace's chosen property-test crate for the facade crate's own dev-dependencies; D-17 mandates reusing the exact pin, not a fresh version resolution |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `paladin-llm` (workspace path dep, `default-features = false`) | `0.10.0` | Brings `Commissary`, `CommissaryPlan`, `Consignment`, `ConsignmentItem`, `ShedItem`, `Stockpile`, `CommissaryError` into `paladin-memory`'s dependency graph | New production dependency edge for this phase (D-01) |

### Alternatives Considered
None — CONTEXT.md D-02 already rejected the only two architectural alternatives (facade-layer
rationing; relocating `Commissary`). No library alternative to `Commissary::dispense` itself was
in scope (`Commissary` is a locked, already-shipped in-house component; PRD explicitly forbids
touching its algorithm this phase).

**Installation:**
```bash
# crates/paladin-memory/Cargo.toml — add to [dependencies]:
#   paladin-llm = { version = "0.10.0", path = "../paladin-llm", default-features = false }
# add to [dev-dependencies]:
#   proptest = "1.4"
```

**Version verification:** `paladin-llm`'s manifest version is `0.10.0` (read directly from
`crates/paladin-llm/Cargo.toml:3`), matching `paladin-memory`'s own version — both are workspace
path dependencies, so no registry lookup applies; `proptest = "1.4"` is copied byte-for-byte from
root `Cargo.toml:246`, already resolved in `Cargo.lock` (confirmed present: `Cargo.lock:4070`).
`cargo-semver-checks 0.50.0` and `cargo-llvm-cov 0.8.7` are the exact versions installed and
callable in this devcontainer (`cargo semver-checks --version` / `cargo llvm-cov --version`,
verified 2026-09-16) — both match the versions CONTEXT.md and CI (`ci.yml`'s `taiki-e/install-action`
steps) already pin, so no drift.

## Package Legitimacy Audit

No new third-party (non-workspace) package is added by this phase. `proptest` is already present
in `Cargo.lock` as a workspace dev-dependency of the root `paladin-ai` crate; adding the identical
pinned version string to a second crate's `[dev-dependencies]` introduces no new entry to the
dependency graph's trust surface, only a second declaring crate. `paladin-llm` is an in-workspace
crate (path dependency), not fetched from any registry.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `proptest 1.4` | crates.io | already resolved workspace-wide | N/A (already vetted, in-tree since an earlier phase) | github.com/proptest-rs/proptest | OK | Approved — no new registry lookup needed, already in `Cargo.lock` |
| `paladin-llm` | workspace path dependency | N/A (in-tree) | N/A | N/A (this repo) | OK | Approved — not a registry package |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
                         PaladinExecutionService::execute()  (facade, application tier)
                                       │
                                       ▼
              retrieve_context_with_timeout(paladin, query, execution_id)
                                       │  tokio::time::timeout(5s, …)
                                       ▼
   ┌───────────────────────────────────────────────────────────────────────────┐
   │  RagRetrievalService::retrieve_context(paladin_id, query)  (paladin-memory)│
   │                                                                           │
   │   1. embedding.embed_text(query)          → SanctumPort query vector     │
   │   2. sanctum.search(query)                → Vec<SanctumSearchResult>     │
   │   3. filter_by_similarity()               → drop below min_similarity   │
   │   4. deduplicate_memories()                → drop near-identical         │
   │   5. rank_by_relevance()                   → sort by score desc         │
   │   6. ration(ranked)  ◄── NEW (D-14, replaces truncate_to_token_budget)  │
   │        │                                                                 │
   │        ▼                                                                 │
   │   ┌─────────────────────────────────────────────────────────────────┐   │
   │   │ build Consignment: one ConsignmentItem per ranked memory,        │   │
   │   │   priority = rank index (D-08), label = memory UUID (D-09)       │   │
   │   │                                                                   │   │
   │   │ Commissary::new("rag", synthetic capabilities{max=budget}, ◄─────┼───┼── NEW edge into
   │   │   counter, plan{reserved=0, fallback=None})           (D-04/D-06)│   │   paladin-llm
   │   │                                                                   │   │
   │   │ commissary.dispense("", &consignment)  → Result<Stockpile, …>    │   │
   │   └─────────────────────────────────────────────────────────────────┘   │
   │        │                                                                 │
   │        ▼  Stockpile{dispensed, shed, prompt_tokens, allotted, exact}     │
   │   build new result struct: retained memories (body = dispensed.body,    │
   │   truncated flag) + shed: Vec<ShedItem> + accounting fields   (D-12)    │
   └───────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
                new-result-struct  →  format_for_prompt(&result)   (D-15: appends
                                       │                              omission marker
                                       ▼                              when shed non-empty)
                          facade's format_retrieved_context(&result)  (D-15: same marker,
                                       │                              second renderer)
                                       ▼
                        injected into the reasoning-loop system prompt
```

### Recommended Project Structure
No new files/directories beyond what CONTEXT.md's canonical refs already name:
```
crates/paladin-memory/src/services/rag_retrieval_service.rs   # ration() seam, dispense adoption, both
                                                                # renderers' marker emission, new result/error
                                                                # types, unit + property tests (same file)
crates/paladin-memory/src/services/mod.rs                      # re-export new result/error types + ShedItem
crates/paladin-memory/src/prelude.rs                            # (optional) crate-root convenience re-export
src/application/services/sanctum/mod.rs                         # facade re-export extension
tests/integration/rag_commissary_test.rs                        # NEW ungated integration test (D-18)
tests/integration/mod.rs                                        # +1 `pub mod rag_commissary_test;` line,
                                                                  # registered beside `in_memory_sanctum_tests`
```

### Pattern 1: Synthetic-capabilities Commissary construction (D-04)
**What:** Build a `Commissary` from a caller-invented `ProviderCapabilities` rather than an
adapter's real one, when the "provider" is actually a budget policy, not an LLM.
**When to use:** Any bounded-allocation problem that wants Commissary's shed/truncate machinery
but has no real `LlmPort` behind it.
**Example (constructed from verified field shapes, not copied from a file that doesn't exist yet):**
```rust
// crates/paladin-memory/src/services/rag_retrieval_service.rs (new code, inside `ration`)
use std::sync::Arc;
use paladin_llm::services::commissary::{Commissary, CommissaryPlan, Consignment, ConsignmentItem};
use paladin_ports::output::llm_port::ProviderCapabilities;

let budget_u32 = u32::try_from(self.config.max_tokens)
    .map_err(|_| RagRetrievalError::BudgetTooLarge { max_tokens: self.config.max_tokens })?;

let capabilities = ProviderCapabilities {
    max_context_tokens: Some(budget_u32),
    ..ProviderCapabilities::default()
};
let plan = CommissaryPlan {
    reserved_completion_tokens: 0,
    fallback_context_tokens: None,
    ..CommissaryPlan::default()
};
let commissary = Commissary::new("rag", capabilities, self.token_counter.clone(), plan)
    .map_err(RagRetrievalError::Commissary)?;

let mut consignment = Consignment::new();
for (rank, result) in ranked.iter().enumerate() {
    consignment.push(ConsignmentItem {
        label: result.entry.memory.id.to_string(),      // D-09: id, never content
        body: result.entry.memory.content.clone(),
        priority: u8::try_from(rank).unwrap_or(u8::MAX), // D-08: rank order
    });
}

let stockpile = commissary.dispense("", &consignment).map_err(RagRetrievalError::Commissary)?;
```
This compiles against the verified `Commissary::new(provider, capabilities, counter, config)`
signature, the verified `CommissaryPlan` field list/defaults, and the verified
`ConsignmentItem { label, body, priority }` field list — all read directly from
`crates/paladin-llm/src/services/commissary.rs` on 2026-09-16.

### Pattern 2: `#[from]` error boundary conversion (house pattern, D-13)
**What:** A new per-module `thiserror` enum with `#[from]` variants for each upstream error,
converted at the crate boundary rather than widening an existing port error.
**When to use:** Any adapter composing two ports/crates that must not leak either's error type
raw, and must not extend a shared port trait's error enum.
**Example, matching the exact shape `CommissaryError` (see below) makes trivial:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum RagRetrievalError {
    #[error("sanctum error: {0}")]
    Sanctum(#[from] paladin_ports::output::sanctum_port::SanctumError),
    #[error("commissary error: {0}")]
    Commissary(#[from] paladin_llm::services::commissary::CommissaryError),
    #[error("rag.max_tokens ({max_tokens}) exceeds u32::MAX")]
    BudgetTooLarge { max_tokens: usize },
}
```
`CommissaryError` derives `Debug, Error, Clone, PartialEq` (verified — see Pitfall 2), so it can
sit behind a `#[from]` variant with no further trait-bound friction.

### Anti-Patterns to Avoid
- **Reaching for `unwrap_or(0)` on the `usize → u32` conversion (D-05):** ADR-0004's stance is
  error, never clamp. `u32::try_from(self.config.max_tokens)` must produce a typed error variant,
  not a silent zero-budget Commissary (which would then error confusingly deep inside `dispense`
  on `FixedMaterialExceedsAllowance` with a useless message).
- **Comparing `Stockpile`/`ShedItem`/`Consignment` values with `assert_eq!` on the whole struct:**
  none of them derive `PartialEq` (verified below) — tests must compare individual fields.
- **Re-typing the omission-marker string in more than one place:** D-15 requires one shared
  helper/constant; a second literal copy in the facade renderer is exactly the kind of drift the
  house pattern (see `WindowSource::as_str`'s substring-contract test in `paladin-llm`) exists to
  prevent.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Priority-ordered shedding + per-item truncation with a marker | A bespoke loop in `rag_retrieval_service.rs` (which is exactly what's being deleted) | `Commissary::dispense` | Already handles the `n == 1` guard, char-boundary-safe truncation (multi-byte tested), stable-sort shedding, and the exact/approximate tally distinction — reimplementing any of this is the anti-pattern the phase exists to retire |
| Context-window precedence resolution | A local `if let Some(...) else ...` chain inside `ration()` | `Commissary::new`'s already-internal call to `paladin_llm::window::resolve_context_window` under `WindowFallbackPolicy::Strict` | `Commissary::new` calls this internally already (verified in `commissary.rs:367-377`) — the RAG service never needs to call the resolver directly, only supply `capabilities`/`config.fallback_context_tokens` |
| `usize` → `u32` conversion with an informative failure | `as u32` (silently wraps) or `.min(u32::MAX as usize) as u32` (silently clamps) | `u32::try_from(x).map_err(...)` | ADR-0004: never clamp; D-05 explicitly forbids clamping here |

**Key insight:** every primitive this phase needs already exists, tested, in `paladin-llm`. The
entire implementation risk is in the *seam* (converting `SanctumSearchResult` ↔ `ConsignmentItem`
↔ the new result struct) and in the *release bookkeeping*, not in any algorithm.

## Common Pitfalls

### Pitfall 1: The cited "lateral dependency precedent" is a dev-dependency, not a production one
**What goes wrong:** CONTEXT.md D-01 describes `crates/paladin-battalion/Cargo.toml:67-69`'s
`paladin-llm` dependency as "direct house precedent" for the new `paladin-memory` → `paladin-llm`
edge. Read in full, `paladin-battalion/Cargo.toml` places `paladin-llm` under
**`[dev-dependencies]`** (confirmed 2026-09-16), with its own comment explaining it exists only
for `MockLlmAdapter` in `paladin-battalion`'s own test code — never shipped in `paladin-battalion`'s
published production dependency graph.
**Why it happens:** Both edges carry the identical "No cycle: paladin-llm depends only on
paladin-core + paladin-ports" comment text, which makes them look like the same commitment at a
skim. They are not: a dev-dependency never appears in the published crate's `Cargo.toml`
`[dependencies]` table that `cargo add`/`cargo install` resolves, so it never bloats a downstream
consumer's build and never has publish-order implications. A *production* dependency does both.
**How to avoid:** Do not describe this phase's edge to a reviewer, in a commit message, or in the
`crate-map.md`/`lib.rs` narrative (D-03) as "the same kind of edge already in the tree" — say
plainly that this is the first *production* lateral adapter→adapter edge of this shape. The edge
is still architecturally sound (the "no cycle" claim is independently true — `paladin-llm`'s own
`Cargo.toml` dependencies are exactly `paladin-core` + `paladin-ports` + optional
`reqwest`/`rand`/`base64`, all `optional = true` and gated by provider features), it is just a
*new* kind of commitment, not a repeat of an existing one — matching D-01's own "Reversibility:
costly" callout, which already telegraphs that this is understood to be a bigger step than a
dev-only edge would be.
**Warning signs:** A semver-discovery run should be watched carefully for `paladin-memory`'s own
published dependency graph gaining `paladin-llm` for the first time ever — `cargo metadata` before
and after the phase's dependency-edge commit is a good sanity check to include in the plan's
verification steps (not currently in CONTEXT.md's gate list, but cheap and directly confirms the
claim).

### Pitfall 2: `Consignment`/`ConsignmentItem`/`ShedItem`/`DispensedItem`/`Stockpile` derive only `Debug, Clone` (no `PartialEq`, no `Serialize`)
**What goes wrong:** A test author reaches for `assert_eq!(stockpile, expected_stockpile)` or a
result-struct field derives `PartialEq`/`Serialize` by inheritance assumption; both fail to
compile. Verified directly from `commissary.rs`: `Consignment` derives `Debug, Clone, Default`;
`ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile` all derive `Debug, Clone` only. Only
`CommissaryError` derives `Debug, Error, Clone, PartialEq`.
**Why it happens:** Most value types in this codebase do derive `PartialEq` (e.g. `RagConfig`
does not either, interestingly, but `RetrievalTrigger` does) — there's no workspace-wide rule, so
assuming it is easy.
**How to avoid:** Property and unit tests must assert on individual fields
(`stockpile.shed[0].label`, `stockpile.shed.len()`, etc.), never on struct equality. If the new
COMM-02 result struct needs `PartialEq` for its own tests, it can derive it locally as long as
every field it wraps also supports it — `ShedItem`/`DispensedItem` do NOT derive `PartialEq`, so a
result struct that embeds them directly cannot derive `PartialEq` either without a manual impl or
comparing sub-fields. Recommend the result struct derive only `Debug, Clone` to match its
constituents, and have tests assert on fields.
**Warning signs:** A `derive(PartialEq)` on the new result struct that fails to compile with
"binary operation `==` cannot be applied to type `ShedItem`" — decide the derive strategy before
writing the first test, not after hitting this.

### Pitfall 3: The four RAG-consuming test suites all assert on the CURRENT `Vec<SanctumSearchResult>` return type and must all migrate together
**What goes wrong:** `rag_retrieval_service.rs`'s own `#[cfg(test)]` module has four tests
(`test_successful_retrieval_with_multiple_memories`, `test_filtering_by_min_similarity`,
`test_format_for_prompt`, `test_empty_results_graceful_handling`) that call `.retrieve_context(...)`
and assert `results.len()`, and one (`test_format_for_prompt`) that calls
`.format_for_prompt(&memories)` where `memories: Vec<SanctumSearchResult>` built directly (not via
retrieval). All five break to a compile error the moment the return type/parameter type changes,
and none of them is mentioned by name in CONTEXT.md's canonical refs.
**Why it happens:** The canonical refs list mentions "the test module's `MockEmbeddingPort`/
`MockSanctumPort`/`create_test_entry` helpers" as reusable, but doesn't enumerate every existing
test that will need a signature update — a planner working strictly from the canonical-refs list
alone could miss that `test_format_for_prompt` builds its `memories` vec directly via
`create_test_entry` (not through `retrieve_context`), so it needs its own local conversion into
the new result-struct's item shape, not just a return-type follow-through.
**How to avoid:** Treat the migration of `rag_retrieval_service.rs`'s existing five tests as an
explicit task, separate from "write new unit tests" — they are pre-existing coverage that must
keep passing, and `test_format_for_prompt` in particular needs a small helper to wrap
`SanctumSearchResult` values into whatever the new result-struct's per-item type is with a
plausible `body`/`truncated` (e.g. `body: memory.content.clone(), truncated: false`) so the
renderer test still exercises real content.
**Warning signs:** `cargo test -p paladin-memory` failing to compile after the result-type change
lands, with five errors all in the same file's test module.

### Pitfall 4: `CHANGELOG.md` has an `[Unreleased]` section carrying Phase 32's facade re-export bullets — not under `[0.10.0]`
**What goes wrong:** `CHANGELOG.md:8-16` is an `## [Unreleased]` header with two "Added" bullets
(the Commissary facade re-export list, and the window-resolver facade re-export list) — both
introduced by commit `2e16fabb` ("regenerate public API baseline for Phase 32 window exports").
COMM-04's own success criterion says the `[0.10.0]` section must carry "the Phase 31/32 API
entries" — the *behavioral*/`Changed` Commissary entry (constructor signature break) IS correctly
under `[0.10.0]` (verified at line ~99-114, not CONTEXT.md's approximate "~82-91" — the exact
range shifted because the `[Unreleased]` section above it grew), but these two `Added` bullets are
not.
**Why it happens:** Since Phase 29 D-18/D-21 established that `0.10.0` is bumped on the feature
branch with no tag yet, there should be no reason for an `[Unreleased]` section to exist above
`[0.10.0]` at all mid-cycle — its presence looks like whoever wrote Phase 32's changelog commit
used the wrong header out of habit (most repos put new entries under `[Unreleased]` by default).
**How to avoid:** D-24's exact gate is "CHANGELOG grep for the RAG note + Phase 31/32 entries" —
a bare `grep` for the entry text succeeds regardless of which header it sits under, so this
technically does NOT block the mechanical gate. But COMM-04's plain-language success criterion
("the `CHANGELOG.md` `[0.10.0]` section carries... the Phase 31/32 API entries") reads as
requiring the *section*, not just presence anywhere in the file. Recommend the phase's COMM-04
plan fold `CHANGELOG.md:9-16`'s two `[Unreleased]` bullets into `[0.10.0]`'s `### Added` block (or
merge them with the RAG `### Added` bullet already planned per D-21) in the same pass that adds
the RAG entries, and delete the now-empty `[Unreleased]` header — cheap, and it removes a genuine
ambiguity about what "the `[0.10.0]` section carries X" means. Flag this decision explicitly
rather than silently deciding it; it is a small scope question, not a locked decision.
**Warning signs:** A grep-only verification of D-24's CHANGELOG gate that reports success while
`[0.10.0]`'s own `### Added` section is still missing the facade re-export note.

### Pitfall 5: The `crate-isolation` CI job already builds `paladin-memory` with `--all-features` and separately with `--no-default-features` — the new edge must survive both
**What goes wrong:** Verified `ci.yml:583-628`: the `crate-isolation` job's `paladin-memory` matrix
entry runs THREE builds — `cargo build -p paladin-memory` (default: no features), `cargo build -p
paladin-memory --no-default-features` (identical to default today, since `default = []`), and
`cargo build -p paladin-memory --all-features` (`sqlite` + `qdrant` + `content-processing`
together). Because `paladin-llm` is added to `[dependencies]` unconditionally (not behind any
`paladin-memory` feature), all three builds now compile `paladin-llm` with
`default-features = false`, and any accidental leak of a `paladin-llm` feature (e.g. someone later
adds `features = ["mock"]` to make a test pass) would silently change `crate-isolation`'s
`--all-features` build without ever touching `paladin-memory`'s own Cargo.toml features list.
**Why it happens:** It's tempting mid-implementation, if a `Commissary`-based test needs the
`mock` feature's `MockLlmAdapter` for convenience, to add `features = ["mock"]` to the new
`paladin-llm` dependency line — this is unnecessary (RAG never needs a real or mock `LlmPort`, only
a synthetic `ProviderCapabilities`, per D-04) but would compile fine and only show up as an
unexpected feature-unification effect on `--all-features` builds elsewhere in the workspace.
**How to avoid:** Keep the dependency line exactly as D-01 specifies — `default-features = false`,
no `features = [...]` list at all — since `commissary` and `window` are unconditional modules,
confirmed by reading `paladin-llm/src/lib.rs`: `pub mod services;` and `pub mod window;` carry no
`#[cfg(feature = ...)]` gate.
**Warning signs:** `cargo build -p paladin-memory --no-default-features` pulling in `reqwest` (a
`cargo tree -p paladin-memory --no-default-features -i reqwest` check make this instantly visible)
would be the tell that the dependency line grew a feature it shouldn't have.

## Code Examples

### Verified `Commissary` construction and dispense signatures
```rust
// Source: crates/paladin-llm/src/services/commissary.rs (read in full 2026-09-16)
pub fn new(
    provider: impl Into<String>,
    capabilities: ProviderCapabilities,
    counter: Arc<dyn TokenCounterPort>,
    config: CommissaryPlan,
) -> Result<Self, CommissaryError>

pub fn dispense(
    &self,
    fixed: &str,
    consignment: &Consignment,
) -> Result<Stockpile, CommissaryError>
```

### Verified `CommissaryPlan` defaults (all fields, `Default` impl)
```rust
// Source: crates/paladin-llm/src/services/commissary.rs:107-119
CommissaryPlan {
    reserved_completion_tokens: 0,
    fallback_context_tokens: None,
    per_item_min_bytes: 0,
    per_item_max_bytes: usize::MAX,
    pessimistic_tokens_per_1000_bytes: 358,
    truncation_marker: "\n... (truncated)".to_string(),
    model_hint: String::new(),
}
```

### Verified `ShedItem` and `Stockpile` field shapes (COMM-02's exact surface)
```rust
// Source: crates/paladin-llm/src/services/commissary.rs:181-225
#[derive(Debug, Clone)]
pub struct ShedItem {
    pub label: String,
    pub priority: u8,
    pub original_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct Stockpile {
    pub dispensed: Vec<DispensedItem>,
    pub shed: Vec<ShedItem>,
    pub prompt_tokens: u32,
    pub allotted_tokens: u32,
    pub exact_tally: bool,
}
```

### Current code being removed (the exact anti-pattern, for the exit-grep baseline)
```rust
// Source: crates/paladin-memory/src/services/rag_retrieval_service.rs:187-212 (BEFORE this phase)
fn truncate_to_token_budget(
    &self,
    results: Vec<SanctumSearchResult>,
) -> Vec<SanctumSearchResult> {
    let mut total_tokens = 0;
    let mut truncated = Vec::new();
    for result in results {
        let estimated_tokens = result.entry.memory.content.len() / 4;
        if total_tokens + estimated_tokens <= self.config.max_tokens {
            total_tokens += estimated_tokens;
            truncated.push(result);
        } else {
            log::debug!("Truncating memories at token budget: {} tokens used of {} max",
                total_tokens, self.config.max_tokens);
            break; // <-- silent drop, no record of what was dropped or why
        }
    }
    truncated
}
```

### Verified existing `with_token_counter` builder shape to mirror (D-07)
```rust
// Source: src/application/services/paladin/paladin_execution_service.rs:849-861
pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounterPort>) -> Self {
    info!("Setting PaladinExecutionService token counter: {}", counter.name());
    self.token_counter = counter;
    self
}
```

### Verified `HeuristicTokenCounter` (the D-07 default)
```rust
// Source: crates/paladin-memory/src/token_counter/heuristic.rs:16-27
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicTokenCounter;

impl TokenCounterPort for HeuristicTokenCounter {
    fn count(&self, text: &str, _model: &str) -> u32 {
        (text.chars().count() as u32).div_ceil(4)
    }
    fn name(&self) -> &str { "heuristic" }
    // is_exact() not overridden -> trait default false, matches D-07's "is_exact() == false"
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `content.len() / 4` estimate, lowest-scoring items silently dropped | `Commissary::dispense` over score-derived priority, shed recorded, per-item marker | This phase (v0.10.0) | ~30% fewer bytes planned per token at the same `max_tokens` (pessimistic ratio 358/1000 vs the old flat /4 == 250/1000 equivalent); oversized single memory now retained-truncated instead of dropped entirely |
| `RagRetrievalService::new(sanctum, embedding, config)` returns `Vec<SanctumSearchResult>` from `retrieve_context` | Same constructor (unchanged, D-06), new result struct return type | This phase | One-way signature break (ADR-0051 clean-break authority for Phases 31-33), MIGRATION.md §9.2 row required |

**Deprecated/outdated:** `truncate_to_token_budget` and its silent-drop behavior — the Phase 26
D-13 deferral this phase closes.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Wording suggestions for the omission marker and `info!` line are illustrative only (Claude's Discretion per D-15/D-16) | Architecture Patterns | None — explicitly discretionary, not a locked fact |
| A2 | The recommendation in Pitfall 4 (fold `[Unreleased]` bullets into `[0.10.0]`) is this researcher's judgment, not a locked decision — CONTEXT.md does not mention the `[Unreleased]` section at all | Common Pitfalls #4 | If the planner disagrees and leaves `[Unreleased]` as-is, the mechanical grep-based D-24 gate still passes; only the plain-language "the [0.10.0] section carries X" reading is at stake |
| A3 | `cargo metadata` diffing as a sanity check for Pitfall 1's "first production edge" claim is a suggested addition, not in CONTEXT.md's gate list | Common Pitfalls #1 | Low — purely additive verification, omitting it does not block any locked gate |

**All other claims in this document were verified directly against the live tree, `Cargo.lock`,
installed tool versions, or `ci.yml`/`Makefile`/`MIGRATION.md`/`CHANGELOG.md` content on
2026-09-16 — none are `[ASSUMED]`.**

## Open Questions

1. **Exact name of the new result struct, error enum, and `ration` seam**
   - What we know: shape is fully locked (D-12/D-13/D-14); names are Claude's Discretion.
   - What's unclear: nothing blocking — this is genuinely open by design.
   - Recommendation: something in the Medieval Military ubiquitous-language register consistent
     with `Stockpile`/`Consignment` naming, e.g. `RagStockpile` or `RationedContext` for the
     result struct, `RagRetrievalError` for the error enum (matches the house pattern's naming:
     `BattalionError`, `PaladinError`), `ration` for the private seam (already suggested in D-14
     itself).

2. **Whether `[Unreleased]` CHANGELOG bullets should fold into `[0.10.0]` (Pitfall 4)**
   - What we know: the mechanical grep gate (D-24) passes either way; the plain-language success
     criterion reads more naturally if they're under `[0.10.0]`.
   - What's unclear: whether this is in scope for COMM-04's plan or an unrelated pre-existing
     defect the phase should leave alone.
   - Recommendation: fold it — it's a two-line CHANGELOG edit in the same commit that's already
     touching `[0.10.0]`'s `### Added` section for the new dependency-edge note, and it removes an
     ambiguity a future reader of the release gate would otherwise have to re-investigate.

3. **Whether to name the `paladin-memory | RagRetrievalService`/`retrieve_context_with_timeout`
   §9.2 rows as one row or two**
   - What we know: D-22 names them as two separate rows (one per `crate | Type` pair per Phase 29
     D-04's keying rule — `RagRetrievalService` the struct/its methods, and
     `retrieve_context_with_timeout` the free function).
   - What's unclear: whether `format_for_prompt`'s own parameter-type break needs its own row or
     folds under the `RagRetrievalService` row (it's an inherent method, same type).
   - Recommendation: fold `format_for_prompt` under the `RagRetrievalService` row — Phase 29 D-04's
     keying is `crate | Type`, not `crate | Type | method`, so every inherent-method break on the
     same struct is one row (matches how `Commissary`'s row already covers both `new` and
     `from_port` together).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo-semver-checks` | COMM-04 semver gate | ✓ | 0.50.0 (matches `ci.yml` pin exactly) | — |
| `cargo-llvm-cov` | COMM-04 coverage gate | ✓ | 0.8.7 (matches `ci.yml` pin exactly) | — |
| Rust toolchain `1.88` (MSRV) | COMM-04 MSRV gate | ✓ | `1.88.0`/`1.88` both installed via rustup | — |
| Rust toolchain (default) | general build | ✓ | `1.97.1` (active default) | — |
| Docker | `make services-up` (needed for `make coverage`'s live Redis/MinIO) | ✗ | — | Coverage evidence must be attributed to the CI `coverage` job, exactly as Phase 29/32 precedent (`docker info` confirmed failing in this devcontainer, 2026-09-16) |
| `mdbook` / `mdbook-linkcheck` | docs build check (D-03/docs edits) | ✓ | both present at `/usr/local/cargo/bin/` | — (CONTEXT.md's hedge "else the docs CI job is the check" is unnecessary — a local `mdbook build docs` can be run directly) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** Docker → CI-attributed coverage evidence (already the
house pattern, not a new gap this phase introduces).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), `#[tokio::test]` for async, `proptest!` macro for COMM-01's property test |
| Config file | none — no `pytest.ini`/`jest.config`-equivalent; feature gates in `Cargo.toml`/`tests/integration/mod.rs` `#[cfg(feature = "...")]` attributes serve this role |
| Quick run command | `cargo test -p paladin-memory --lib` (unit + property tests, fast, no services) |
| Full suite command | `cargo test --features integration-tests,llm-all --workspace -- --test-threads=1` (mirrors `scripts/coverage.sh`'s exact invocation) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COMM-01 | Retained total ≤ budget; highest-scoring retained | property (`proptest!`) | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ Wave 0 — new test to write in this phase |
| COMM-01 | `Commissary::dispense` called with score-derived priority over `rag.max_tokens` | unit (example-based, distinct non-round numbers per house style) | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ Wave 0 |
| COMM-02 | `ShedItem` list present when budget exceeded, absent when everything fits | unit | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ Wave 0 |
| COMM-02 | Truncation marker present/absent matching `shed` non-emptiness, in both renderers | unit (one per renderer: `paladin-memory`'s `format_for_prompt`, facade's `format_retrieved_context`) | `cargo test -p paladin-memory --lib` and `cargo test -p paladin-ai paladin_execution_service` | ❌ Wave 0 (both renderer call sites already exist, but no marker test exists yet) |
| COMM-03 | Real RAG path exercises `Commissary::dispense` end-to-end, ungated | integration | `cargo test --test integration_tests rag_commissary_test` (via `tests/integration/mod.rs`'s aggregator — exact top-level test-binary name confirmed by `ls tests/*.rs`) | ❌ Wave 0 — new file `tests/integration/rag_commissary_test.rs` |
| COMM-03 | No silent token-based truncation remains, grep-provable | automated grep, not `cargo test` | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` (expect 0 non-comment hits after rewrite) and `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` (expect empty) | N/A — script-based check, not a test file |
| COMM-04 | `MIGRATION.md` no-TBD, §9.2 matches allowlist row-for-row | automated | `grep -c TBD MIGRATION.md` (expect 0) && `bash scripts/check-migration-allowlist.sh` (verified script exists and is runnable locally, 2026-09-16) | ✓ script exists |
| COMM-04 | Backward-compat frozen tests pass | integration | `cargo test --features web-server --test v0_9_config_boot` && `cargo test -p paladin-web --test openapi_golden_v0_9` | ✓ both test files pre-exist (Phase 29) |
| COMM-04 | semver/MSRV/publish-dry-run green | automated (CLI tools, not `cargo test`) | see D-24's exact command list; all tools confirmed installed locally (Environment Availability above) | ✓ |
| COMM-04 | 82% coverage floor | automated | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` (needs Docker; CI-attributed here) | N/A locally — CI job exists |

### Sampling Rate
- **Per task commit:** `cargo test -p paladin-memory --lib` (fast, no services, catches the
  rationing-logic regressions immediately)
- **Per wave merge:** `cargo test --workspace` (default features; add `--all-features` once wave 1's
  dependency edge lands, to catch the `crate-isolation`-equivalent break locally before CI does)
- **Phase gate:** Full COMM-04 gate list (D-24) on the final commit, evidence recorded per D-25

### Wave 0 Gaps
- [ ] `proptest = "1.4"` added to `crates/paladin-memory/Cargo.toml`'s `[dev-dependencies]`
      (D-17) — no property-test infrastructure exists yet in this crate
- [ ] `tests/integration/rag_commissary_test.rs` — new ungated integration test (D-18), registered
      in `tests/integration/mod.rs` beside line 55's `in_memory_sanctum_tests`
- [ ] The five pre-existing tests in `rag_retrieval_service.rs`'s `#[cfg(test)]` module must be
      migrated to the new result-struct signature in the SAME commit the signature changes
      (Pitfall 3) — not a new gap, but a pre-existing-coverage migration that must not be dropped

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | no | not touched by this phase |
| V3 Session Management | no | not touched by this phase |
| V4 Access Control | no | not touched by this phase |
| V5 Input Validation | yes (narrow) | `u32::try_from` on the `usize → u32` budget conversion (D-05), error not clamp; `RagConfig::validate` already rejects `max_tokens == 0` |
| V6 Cryptography | no | not touched by this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Sensitive memory content leaking into logs/labels | Information Disclosure | D-09: `ConsignmentItem.label`/`ShedItem.label` carry the memory UUID, never `content`; the `info!` line (D-16) logs counts and ids only, matching the house rule already applied to `Commissary`'s own `Debug` impl (which logs `counter.name()`, never counted text) |
| Integer overflow/underflow on the budget conversion | Denial of Service (degraded/incorrect budgeting) | `u32::try_from` with a typed error (D-05), not `as` cast or saturating conversion — an oversized `rag.max_tokens` fails loudly at retrieval time rather than silently wrapping to a tiny or huge budget |

No new attack surface is introduced: this phase adds no network-facing code, no new
authentication/authorization path, and no new cryptographic operation. The one narrow V5 item
above is already covered by locked decisions (D-05) — nothing new for the planner to design here.

## Sources

### Primary (HIGH confidence — read directly from the live tree, 2026-09-16)
- `crates/paladin-llm/src/services/commissary.rs` — full file read; `Commissary`, `CommissaryPlan`,
  `Consignment`, `ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryError`
  field shapes, derives, defaults, and all 18 unit tests
- `crates/paladin-llm/src/window.rs` — full file read; `resolve_context_window`,
  `WindowFallbackPolicy`, `WindowSource`
- `crates/paladin-memory/src/services/rag_retrieval_service.rs` — full file read; current
  `retrieve_context`, `truncate_to_token_budget`, `format_for_prompt`, `retrieve_context_with_timeout`,
  and the complete test module
- `crates/paladin-memory/src/config/rag.rs` — full file read; `RagConfig`, `RetrievalTrigger`,
  `MemoryExtractionStrategy`
- `crates/paladin-ports/src/output/token_counter_port.rs` — full file read; `TokenCounterPort` trait
- `crates/paladin-memory/src/token_counter/heuristic.rs` — full file read; `HeuristicTokenCounter`
- `crates/paladin-llm/src/lib.rs`, `crates/paladin-llm/Cargo.toml`, `crates/paladin-llm/src/services/mod.rs`
  — confirmed featureless build viability and unconditional `commissary`/`window` modules
- `crates/paladin-battalion/Cargo.toml` — confirmed the `paladin-llm` edge is `[dev-dependencies]`
  (Pitfall 1's correction)
- `crates/paladin-memory/Cargo.toml` — current dependency/dev-dependency lists
- `src/application/services/paladin/paladin_execution_service.rs` — lines ~1297-1330, ~1900-1985,
  ~820-861 read; RAG call site, `retrieve_context_with_timeout`, `format_retrieved_context`,
  `with_token_counter`
- `src/application/services/sanctum/mod.rs` — full file read; facade re-export surface
- `crates/paladin-memory/src/services/mod.rs`, `crates/paladin-memory/src/prelude.rs`,
  `crates/paladin-memory/src/lib.rs` — full/partial reads; current re-export sites
- `tests/integration/mod.rs` — gating confirmed for `in_memory_sanctum_tests` (ungated),
  `rag_integration_tests`/`qdrant_sanctum_tests` (both `#[cfg(feature = "qdrant")]`)
- `tests/integration/rag_integration_tests.rs` — mock `EmbeddingPort` pattern (~lines 204-255),
  three `retrieve_context`/`format_for_prompt` call sites (~lines 300-430) confirmed
- `tests/integration/in_memory_sanctum_tests.rs` — `InMemorySanctum::new(max_entries)` constructor
  pattern confirmed
- `crates/paladin-memory/src/sanctum/in_memory_adapter.rs` — `InMemorySanctum`/`InMemorySanctumConfig`
  constructors confirmed
- `MIGRATION.md` (lines 160-280 read in full) — §9.2 table, the `paladin-llm | Commissary` row
  (line 205) confirmed as the `N/A`-template
- `.cargo/semver-checks-allowlist.toml` — schema and existing entries read
- `scripts/check-migration-allowlist.sh` — full file read; exact awk/grep logic confirmed runnable
  locally
- `CHANGELOG.md` (lines 1-120 read) — `[Unreleased]` section (lines 8-16) and `[0.10.0]` Commissary
  bullet (lines 99-114) located precisely; discrepancy documented in Pitfall 4
- `.github/workflows/ci.yml` — `msrv` (251-267), `semver` (303-430), `crate-isolation` (583-628),
  `coverage` (1344-1400+), `publish-dry-run` (1974-1996) jobs read in full
- `Makefile` — `check-gates`, `coverage`, `api-surface`/`api-surface-update`, `security`,
  `clean-code`, `publish-dry-run` target locations confirmed
- `.project/current-exports.txt` (lines 3955-3980) — RAG re-export baseline confirmed at the cited
  line range
- `docs/src/architecture/commissary.md`, `docs/src/architecture/crate-map.md` (mermaid, no
  `mem --> llm` edge yet — confirmed the D-03 edit is genuinely needed),
  `docs/src/getting-started/configuration.md` (lines 278-290, 405-420),
  `docs/src/api-reference/upgrading.md`/`migration-guide.md` ("Token primitives" subsections located)
- `examples/paladin_with_rag.rs` (lines 110-130) — confirmed this is printed documentation text,
  not compiled example code exercising the changed signatures (D-06's "unchanged" claim doesn't
  need this file touched)
- `crates/paladin-battalion/src/conclave_execution_service.rs` (lines 429-460) — `truncate_output`
  confirmed already marks its cut with `... [truncated]`
- `crates/paladin-content/src/services/content_summarizer_service.rs` — confirmed character-length
  based, not token-budget based
- Live tool checks: `rustc --version` (1.97.1), `cargo --version` (1.97.1), `rustup toolchain list`
  (`1.88`, `1.88.0`, `1.97.1` all present), `cargo semver-checks --version` (0.50.0), `cargo
  llvm-cov --version` (0.8.7), `docker info` (fails — not available), `which mdbook
  mdbook-linkcheck` (both present)
- `grep -rn 'truncate_to_token_budget' crates src docs examples benches` and
  `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` — run directly, baseline captured in
  COMM-03's requirement-support row above

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` — Phase 29/32/28/26 decision summaries relied on for cross-phase precedent
  (not independently re-verified byte-for-byte against every cited phase's own CONTEXT.md, only
  cross-checked where directly relevant to this phase's gates)

### Tertiary (LOW confidence)
- None — every substantive claim above traces to a direct file read or tool invocation performed
  in this session.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new external package; the one dev-dependency addition is an
  already-resolved, already-in-`Cargo.lock` workspace crate
- Architecture: HIGH — every type/signature cited was read from the live source file, not
  recalled from training data or CONTEXT.md's own prose
- Pitfalls: HIGH — all five pitfalls above are grounded in a direct discrepancy or gap found
  during this session's verification pass (dev-vs-prod dependency, missing derives, untracked
  test migrations, CHANGELOG section placement, feature-unification risk), not speculative

**Research date:** 2026-09-16
**Valid until:** This is an internal refactor phase with no external-ecosystem drift risk; the
research remains valid until the tree changes under it. Recommend treating it as stale the moment
any of Phase 30/31/32's already-closed work is touched again, or if `cargo-semver-checks`/
`cargo-llvm-cov` are upgraded before this phase's COMM-04 plan executes (re-check `--version`
output against this document's Environment Availability table first).
