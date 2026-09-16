# Phase 33: Commissary In-Tree Adoption - Context

**Gathered:** 2026-09-16
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected; every question resolved to the recommended option and logged in `33-DISCUSSION-LOG.md`)

<domain>
## Phase Boundary

Give `Commissary` its first production caller and remove the last silent token-truncation
path in the tree:

1. **RAG rations through the Commissary.** `RagRetrievalService::truncate_to_token_budget`
   (`crates/paladin-memory/src/services/rag_retrieval_service.rs:187-212` — inline
   `content.len() / 4`, lowest-scoring memories dropped with a `debug!` line and no record) is
   replaced by a `Commissary::dispense` call over a `Consignment` built from the retrieved
   memories, priority derived from relevance score, budget `rag.max_tokens` (COMM-01).
2. **Nothing is dropped silently.** The `ShedItem` list is surfaced through the RAG result
   path and a truncation marker is emitted in the rendered prompt whenever content was shed;
   tests assert both present when the budget is exceeded and both absent when everything fits
   (COMM-02).
3. **The F4 evidence.** An integration test drives `Commissary::dispense` through the real
   RAG path, and the exit grep proves no silent token-based truncation remains in-tree — the
   Phase 26 D-13 deferral closes (COMM-03).
4. **Re-seal the Phase 29 release gates on the final commit** — `MIGRATION.md` no-TBD and
   §9.2 row-level set-equal with the allowlist, `v0_9_config_boot`, the OpenAPI golden diff,
   `cargo semver-checks`, the MSRV check, `cargo publish --dry-run` in dependency order, and
   `CHANGELOG.md` `[0.10.0]` carrying the RAG truncation-marker note plus the Phase 31/32
   entries — with evidence appended to the Phase 29 acceptance audit, not a new audit
   (COMM-04). This is what lets the `0.10.0` tag be cut (ADR-0051).

**Behavioural change, plus one clean signature break** — governed by ADR-0051 (X-03
superseded for Phases 31-33 only). `retrieve_context`'s return type changes to a result
struct that carries the shed record; no forwarding method, no `#[deprecated]` alias.

**Not in this phase:** changing RAG retrieval, scoring, filtering or deduplication (PRD §4);
any change to `Commissary`'s dispensing algorithm or its public surface (new knobs are
deferred ideas); wiring `HistoryTrimmer` onto the shared resolver (done in Phase 32 PRIM-04 —
this phase only re-runs the regression check); Treasurer, pricing, `cost_estimate`,
allowances, pacing (Milestone 14, ADR-0050); renaming any token type (Phase 30 D-17);
migrating other, already-marked truncation sites (Conclave `truncate_output`) onto the
Commissary.

</domain>

<decisions>
## Implementation Decisions

### Crate seam — where the `dispense` call lives (COMM-01)
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

### Building a Commissary against a bare `rag.max_tokens` budget (COMM-01)
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

### Priority derivation from relevance score (COMM-01)
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

### Dispensing semantics for RAG (COMM-01, COMM-02)
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

### Result shape and error type — the COMM-02 surface
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

### Marker emission and observability (COMM-02)
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

### Test strategy and the COMM-03 exit grep
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

### Release bookkeeping and the COMM-04 re-seal
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

### Folded Todos
- **Verify local `make coverage` reproduces CI's 82.39 % figure**
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6) —
  folded as a verification note only, exactly as Phases 31 and 32 did: COMM-04 already
  requires the coverage floor to be green, so the plan that records coverage evidence also
  records whether the local run reproduces the CI number. No new scope; the todo keeps its
  no-`resolves_phase` status.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone corpus and governing decisions
- `.project/Milestone_13-Token-Economy/Epic_4/prd-commissary-in-tree-adoption.md` — the source
  PRD: R1/R2 (this phase), R3 (folded into Phase 32 — regression check only), R4 (CHANGELOG),
  §4 out of scope, §5 tests, §6 exit criteria.
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` — §0 locked
  terms, §3 verified anchors (RAG silent-truncation row), §4 findings F4/F6 and decision D-7,
  §5 clean-break policy, §7 out of scope.
- `.planning/decisions/0049-commissary-design-and-rename.md` — the Commissary design record
  (`dispense` contract, fail-loud / never-silent) that names Phase 33 as the first caller.
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — clean break for
  Phases 31-33; every break still gets a §9.2 row + allowlist row; the `0.10.0` tag waits on
  COMM-04.
- `.planning/decisions/0050-treasurer-reservation.md` — what stays reserved for Milestone 14.
- `.project/v0.10.0/00-program-overview.md` §3 X-10 (semver hygiene, still governing), X-03
  (superseded here).

### Planning record
- `.planning/ROADMAP.md` — Phase 33 entry (goal, success criteria 1-4, ~line 875) and the
  2026-09-14 extension footer (~lines 1243-1251: COMM-04 origin; Epic 4 R3 folded into
  PRIM-04).
- `.planning/REQUIREMENTS.md` — COMM-01…04 (~lines 449-468) and the traceability rows
  (~574-577).
- `.planning/phases/32-unified-token-primitives/32-CONTEXT.md` — D-01…D-05 (resolver;
  Commissary passes no config table), D-06…D-08 (`is_exact` on the port; `Commissary::new`
  signature), D-13/D-14 (equivalence fixtures; empirical semver discovery with
  `--release-type minor` and the feature-gated second run), D-15/D-16 (§9.2 row keying;
  CHANGELOG format), `<specifics>` (discovery commands).
- `.planning/phases/32-unified-token-primitives/32-05-SUMMARY.md` — the gate-evidence and
  semver-discovery template (six runs, the `qdrant-client` drift workaround, the
  zero-lint-pair rule).
- `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` — D-27 (empirical lint
  discovery), D-28 (CHANGELOG `[0.10.0]`), `<specifics>` (distinct non-round test figures).
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md` — D-03
  (Commissary framing), D-06 (the mdBook usage sketch mirrors a real test), D-17 (no
  renames), D-18 (write "v0.10.0"), `<specifics>` (commit mechanics).
- `.planning/phases/29-program-gates-release/29-CONTEXT.md` — D-04 (row-level allowlist ↔
  §9.2 set-equality), D-07/D-08/D-09 (the two compat test targets), D-10 (audit lives in the
  corpus), D-12 (findings recorded, fix set bounded), D-17 (human-only sign-off), D-18/D-21
  (tag on the `main` merge; record both SHAs).
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — the pointer file that
  gets the re-seal note (D-25).
- `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md` — the evidence-record shape
  (`## Local sweep` table columns `# | Command | Result | Verdict`; `## CI-run table`) that
  `33-CI-EVIDENCE.md` copies.
- `.project/v0.10.0/09-program-acceptance-audit.md` — the corpus audit; sections 1-10 exist
  (§10 "Release readiness" at ~line 1291, its four subsections are the gate list to re-run);
  §11 is appended by this phase.
- `.planning/phases/26-agent-runtime-enhancements/26-11-PLAN.md` (D-13 truths, lines ~23-28)
  — the deferral this phase closes ("the pre-existing inline `content.len() / 4` … is left
  exactly as it is under X-03 and is recorded as a Deferred Idea"); history, do not edit.

### Migration register, semver tooling and release gates
- `MIGRATION.md` §9.2 (~lines 162-277) — row format; the `paladin-llm | Commissary` row
  (line 205, the `N/A`-with-tool-gap template) and the `paladin-memory | TokenCounter` /
  `TokenCounterFactory` rows (203-204, the `Y`-with-allowlist template).
- `.cargo/semver-checks-allowlist.toml` — `[[entry]]` schema (`crate`, `lint`,
  `migration_row`, `requirement_id`, `justification`); 18 entries today.
- `scripts/check-migration-allowlist.sh` — the row-level set-equality check
  (`make check-migration-allowlist`, part of `make check-gates`).
- `scripts/publish-crates.sh` — `CRATES` dependency-ordered array (line 144: `paladin-llm`
  precedes `paladin-memory`).
- `Makefile` — `check-gates` (215), `api-surface` / `api-surface-update` (377-383),
  `clean-code` (419), `publish-dry-run` (566-570), `openapi` (371).
- `.github/workflows/ci.yml` — `msrv` (251), `semver` (303; per-package command ~357;
  row-level step ~361-430), `integration-tests` (632), `coverage` (1344),
  `publish-dry-run` (1974; `main`-push only), `crate-isolation` (583;
  `--no-default-features` build per crate).
- `CHANGELOG.md` — `[Unreleased]` (line 8) and `[0.10.0] - 2026-09-10` (line 18) with its
  `### Behavioral changes` / `### Changed` subsections; the Phase 32 Commissary bullet at
  ~82-91 and the Phase 31 `TokenUsage` bullets at ~70-77 are the entries to verify present.
- `.project/current-exports.txt` — the API-surface baseline (RAG re-exports at 3962-3973;
  `with_rag_retrieval` at 2757).

### Code: the RAG path being changed
- `crates/paladin-memory/src/services/rag_retrieval_service.rs` (~470 lines) — module doc
  bullets, `retrieve_context` (~85-119: the four post-processing steps, `truncate_to_token_budget`
  at 116), `rank_by_relevance` (~172), `truncate_to_token_budget` (187-212, the code being
  removed), `format_for_prompt` (~223-251), `retrieve_context_with_timeout` (~257-280), the
  test module's `MockEmbeddingPort` / `MockSanctumPort` / `create_test_entry` helpers.
- `crates/paladin-memory/src/config/rag.rs` — `RagConfig` (`max_tokens: usize`, `top_k`
  capped at 100 by `validate`, `max_tokens == 0` rejected).
- `crates/paladin-memory/src/services/mod.rs:12` and `crates/paladin-memory/src/prelude.rs:25`
  — re-export sites for the new result/error types; `crates/paladin-memory/src/lib.rs:1-45`
  — crate narrative to extend (D-03).
- `crates/paladin-memory/Cargo.toml` — `[dependencies]` (add `paladin-llm`,
  `default-features = false`), `[dev-dependencies]` (add `proptest = "1.4"`).
- `crates/paladin-memory/src/token_counter/heuristic.rs` — `HeuristicTokenCounter`
  (`chars / 4` rounded up, `is_exact` default `false`); `garrison/token_counter.rs` —
  `TiktokenCounter` (`content-processing`).
- `crates/paladin-memory/src/sanctum/in_memory_adapter.rs:73` — `InMemorySanctum`, the
  integration test's store.
- `crates/paladin-ports/src/output/sanctum_port.rs` — `SanctumSearchResult { entry, score:
  f32 }` (568-581), `SanctumError` variants (284+; not extended).
- `crates/paladin-core/src/platform/container/sanctum.rs:58-73` — `Memory { id: Uuid,
  content, … }` (the label source, D-09).

### Code: the Commissary being adopted
- `crates/paladin-llm/src/services/commissary.rs` — `CommissaryPlan` (63-119),
  `ConsignmentItem` priority contract (121-136), `Consignment` (138-166), `DispensedItem`
  (168-179), `ShedItem` (181-192), `Stockpile` (194-225), `CommissaryError` (227-292),
  `Commissary::new` (344-394), `dispense` algorithm doc + body (431-558), the module doc
  sentence to rewrite (22-25).
- `crates/paladin-llm/src/window.rs` — `resolve_context_window` / `WindowFallbackPolicy` the
  synthetic-capabilities construction (D-04) goes through.
- `crates/paladin-llm/Cargo.toml` — features (`default = ["openai", "mock"]`; `commissary`
  is unconditional).
- `crates/paladin-battalion/Cargo.toml:67-69` — the lateral `paladin-llm` dependency
  precedent and its "no cycle" comment (copy the shape).

### Code: the facade consumer
- `src/application/services/paladin/paladin_execution_service.rs` — the RAG call site
  (~1297-1330: `retrieve_context_with_timeout` → `format_retrieved_context`, the
  `_memories_retrieved_count` and `info!` line), `retrieve_context_with_timeout`
  (1923-1962: error mapping to keep), `format_retrieved_context` (1969-1985: the second
  renderer that emits the marker).
- `src/application/services/sanctum/mod.rs:7-8, 21` — facade re-exports of the RAG types
  (extend with the new result/error types and `ShedItem`).
- `tests/integration/mod.rs` — `in_memory_sanctum_tests` (line 55, ungated) and
  `rag_integration_tests` (line 78, `qdrant`-gated); register the new test beside the former.
- `tests/integration/rag_integration_tests.rs` — the mock `EmbeddingPort` pattern (~200-255)
  and the three `retrieve_context`/`format_for_prompt` call sites (313-430) that migrate to the
  new result type.
- `examples/paladin_with_rag.rs:121-123` — the printed `RagRetrievalService::new` snippet
  (unchanged: the constructor signature is preserved, D-06).

### Docs being edited
- `docs/src/architecture/commissary.md` — gains an "In-tree caller: RAG" section whose sketch
  mirrors the D-18 integration test (Phase 30 D-06 rule); its `Stockpile`/`ShedItem` table
  rows are the vocabulary to reuse.
- `docs/src/architecture/crate-map.md` — mermaid graph (~27-60) and `paladin-memory` section
  (~166).
- `docs/src/getting-started/configuration.md:283` (`max_tokens: 2000 # Max tokens to inject
  from RAG`) and the four-meanings table row at 413 — add the marker/shed sentence.
- `docs/src/api-reference/upgrading.md` and `docs/src/api-reference/migration-guide.md` —
  the "Token primitives" subsection Phase 32 added gets a RAG line pointing at the §9.2 rows.
- `docs/book.toml` (`warning-policy = "error"`), `.github/workflows/docs.yml` — the docs
  build that must stay green.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Commissary::dispense` already does everything COMM-01/02 need — stable priority sort,
  shed-until-fits, char-boundary-safe cut with marker, `ShedItem` record, measured tally
  through the injected counter. The RAG change is a thin adapter: build the `Consignment`,
  call `dispense`, unpack the `Stockpile`.
- `HeuristicTokenCounter` lives in `paladin-memory` — the default counter (D-07) needs no
  new dependency; `TiktokenCounter` is the exact alternative behind `content-processing`.
- `rag_retrieval_service.rs`'s test module already has `MockEmbeddingPort`,
  `MockSanctumPort { results }` and `create_test_entry(paladin_id, content, importance,
  score)` — the seams for the example-based and property tests.
- `tests/integration/rag_integration_tests.rs`'s deterministic mock `EmbeddingPort` and
  `in_memory_sanctum_tests.rs`'s ungated registration are the templates for the D-18 test.
- `MIGRATION.md:203-205`, the Phase 32 allowlist entries, `32-05-SUMMARY.md`'s six-run
  discovery record and `29-CI-EVIDENCE.md`'s tables are copy templates for D-22/D-24/D-25.
- `PaladinExecutionService::with_token_counter` is the builder shape D-07 mirrors.

### Established Patterns
- Lateral adapter-crate dependencies exist and are documented inline with a "no cycle"
  justification (`paladin-battalion` → `paladin-llm`); `default-features = false` is how a
  consumer avoids pulling optional HTTP stacks.
- Errors are per-module `thiserror` enums with `#[from]` conversions at boundaries; ports are
  never widened for an adapter's convenience (`SanctumError` stays as is).
- Nothing budget-shaped is silent: Conclave, redaction and trace paths all append explicit
  `[truncated]`-style markers — RAG was the outlier.
- Sensitive text never reaches logs: labels are ids, log lines carry counts.
- Semver-breaking rows and allowlist entries land in the same commit as the code that
  breaks; lint discovery is empirical with `--release-type minor`; the API-surface baseline
  is regenerated in the same commit (pre-push gate).
- Gate evidence is recorded honestly, including pre-existing reds (`cargo doc`) that are not
  gates, with the exact command per row.

### Integration Points
- `paladin-memory` → `paladin-llm` (new edge): `Commissary`, `CommissaryPlan`, `Consignment`,
  `ConsignmentItem`, `ShedItem`, `CommissaryError` consumed by `rag_retrieval_service.rs`.
- `RagRetrievalService::retrieve_context` → new result struct → facade
  `retrieve_context_with_timeout` / `format_retrieved_context` → system-prompt injection; the
  facade re-export module and `.project/current-exports.txt` follow.
- Release register: `MIGRATION.md` §9.2 ↔ `.cargo/semver-checks-allowlist.toml` ↔ `ci.yml`
  `semver` job ↔ `CHANGELOG.md` `[0.10.0]` ↔ `.project/v0.10.0/09-program-acceptance-audit.md`
  §11 ↔ the `0.10.0` tag (cut on the `main` merge, outside this phase).

</code_context>

<specifics>
## Specific Ideas

- The D-10(a) edge (one memory larger than the whole budget → retained, cut, marked) gets its
  own named test in both the unit and integration suites; its expected output is the old
  behaviour's exact opposite (empty vs. marked excerpt), so the CHANGELOG note names it.
- Property-test strategy sketch: `prop::collection::vec((".{1,600}", 0.0f32..=1.0f32),
  0..=20)` plus `budget in 1u32..=2_000`; shuffle before ranking so the "highest-scoring
  retained" property is tested against unsorted input.
- Use distinct, non-round numbers in the example-based tests (budget `1_234`, bodies of
  `987` / `654` / `321` bytes) so a swapped priority cannot pass by coincidence.
- The RAG-level marker helper lives in `paladin-memory` and is `pub` so the facade renderer
  and the integration test import it rather than re-typing the string.
- `commissary.rs:22-25` rewrite: "the anti-pattern this module was built to replace —
  `RagRetrievalService::truncate_to_token_budget`'s silent drop — was retired in v0.10.0 when
  RAG became this module's first production caller (Phase 33)".
- Semver discovery commands (Phase 32 D-14 shape): `cargo semver-checks check-release
  --package paladin-memory --default-features --baseline-version 0.9.0 --release-type minor`
  and the same with `--features content-processing`; the same pair for `paladin-ai`;
  `--default-features` only for `paladin-llm` (no change expected — confirm empirically).
- `33-CI-EVIDENCE.md` row 1 is the exit grep (D-19), so the F6 closure is the first thing a
  reader sees.

</specifics>

<deferred>
## Deferred Ideas

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

### Reviewed Todos (not folded)
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.6)
  — matched only on generic keywords ("test, crates, paladin"); it is an object-storage
  infrastructure evaluation with no overlap with RAG rationing or the release gates. Folding
  it would violate the scope guardrail, so the mechanical ≥ 0.4 auto-fold rule was
  deliberately not applied (same call as Phase 32). It stays pending with no
  `resolves_phase` tag, as its own text requires.

</deferred>

---

*Phase: 33-commissary-in-tree-adoption*
*Context gathered: 2026-09-16 via `--auto`*
