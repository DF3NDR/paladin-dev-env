# Phase 36 -- Evidence Ledger

Toolchain: `cargo 1.97.1` / `rustc 1.97.1`.

This file is the running house `NN-EVIDENCE.md` for Phase 36 (D-25): a Baseline section
capturing the phase's starting state once, then one section per plan appending its own
per-crate/per-example proof as fixes land. Verbatim captures live under `36-evidence/`;
this file summarizes and points at them.

## Baseline (captured by plan 36-01, D-01)

HEAD at capture: `8fab0788b122e8297264453c3e306d0d62aa4309` (dispatch-time HEAD for plan
36-01; differs from `36-RESEARCH.md`'s research-time re-baseline HEAD
`619551a781f6e352fac896d5fab1ba8630876585` only by four `.planning/`-only phase-planning
commits -- no library source, no `crates/doc-examples` change between the two).

- `cargo doc --workspace --no-deps` (ci.yml:63 verbatim): exit 0, 73 total `warning:`
  lines (65 content diagnostics across 8 crates + 8 per-crate summary lines) ->
  `! grep -q "warning:"` fails, so the effective ci.yml:63 command is RED (exit 1) as
  expected pre-fix. Byte-identical to `34-AUDIT.md` and `36-RESEARCH.md`'s re-baseline.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`: exit 101,
  matching `36-RESEARCH.md`'s re-baseline exit 101 exactly.
- Zero drift confirmed: no `RD-144`+/`EX-123`+ rows minted; `34-AUDIT.md` sec6 remains the
  valid, unmodified work list for this phase.

Full verbatim capture: `36-evidence/36-01-baseline.txt`.

## Plan 36-01 (this plan)

Closes: RD-01, RD-66, RD-126 (paladin-memory), RD-51, RD-127 (paladin-ports), RD-46,
RD-128 (paladin-storage). Adds: EX-109, EX-111, EX-112, EX-113, EX-114, EX-115 via
`examples/token_economy_commissary.rs` and its `examples/README.md` section.

### Closure table (D-24)

| ID | file:line (cited, `34-AUDIT.md`) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-01 | `crates/paladin-memory/src/token_counter/mod.rs:3` | same | unresolved link (bare shorthand to a same-file `pub use` re-export) | explicit markdown link + full crate path: `` [`HeuristicTokenCounter`](crate::token_counter::heuristic::HeuristicTokenCounter) `` | rustdoc-fix commit (see git log `docs(36): resolve rustdoc link in paladin-memory`) |
| RD-66 | same location group (default-feature follower) | same | closed by the RD-01 fix | same | same |
| RD-126 | same location group (all-features follower) | same | closed by the RD-01 fix | same | same |
| RD-51 | `crates/paladin-ports/src/output/structured_executor_port.rs:158` | same | private intra-doc link (`repair_prompt`) | de-linked to plain code font, reworded to name the crate-private helper | `docs(36): resolve rustdoc link in paladin-ports` |
| RD-127 | same location group (all-features follower) | same | closed by the RD-51 fix | same | same |
| RD-46 | `crates/paladin-storage/src/waypoint/contract_tests.rs:673` | same | private intra-doc link (`muster_progress_fixture`) | de-linked to plain code font | `docs(36): resolve rustdoc link in paladin-storage` |
| RD-128 | same location group (all-features follower) | same | closed by the RD-46 fix | same | same |
| EX-109 | gap row | `examples/token_economy_commissary.rs` | new example | `Commissary::new` post-PRIM-02 signature demonstrated | `docs(36): add token_economy_commissary example` |
| EX-111 | gap row | `examples/token_economy_commissary.rs` | new example | Anthropic-shaped `TokenUsage` (cache-read/cache-write included in `prompt_tokens`) via `MockLlmAdapter` | same |
| EX-112 | gap row | `examples/token_economy_commissary.rs` | new example | `TokenCounterPort::is_exact` contrasted (heuristic vs. exact) | same |
| EX-113 | gap row | `examples/token_economy_commissary.rs` | new example | exactness read live from the counter instance, no constructor argument | same |
| EX-114 | gap row | `examples/token_economy_commissary.rs` | new example | `resolve_context_window` + `ResolvedWindow` | same |
| EX-115 | gap row | `examples/token_economy_commissary.rs` | new example | `WindowSource` + `WindowFallbackPolicy` printed | same |

### Verification evidence

- Per-crate sweep (post-fix, D-10): `36-evidence/36-01-percrate.txt` --
  `paladin-memory`, `paladin-ports`, `paladin-storage` each documenting warning-free
  under default features and exiting 0 under
  `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps`.
- `grep -rn 'allow(rustdoc::private_intra_doc_links)'` across all three crates: no
  output (no lint suppression added).
- `grep -rn 'pub fn repair_prompt' crates/paladin-ports/src`: no output (visibility not
  widened).
- `git diff` for all three fixed files: doc-comment lines only.
- Example run (D-16, D-29): `36-evidence/36-01-example-run.txt` -- exit 0 with
  `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/`DEEPSEEK_API_KEY` all unset; stdout names
  `Commissary`, `is_exact`, `resolve_context_window`, `WindowSource`, `prompt_tokens`;
  zero `Quartermaster` mentions.
- `examples/README.md`: `## Token Economy Examples` section present with a matching TOC
  bullet, the `### [token_economy_commissary.rs](token_economy_commissary.rs)` header,
  a `**Demonstrates:**` line, a fenced run command, and no `**Code snippet:**` block
  (D-21).
- `make api-surface`: unchanged, verified before each of this plan's five commits
  (3959 items throughout).
- `cargo fmt --all -- --check`: clean.
- `cargo check --workspace --all-targets --all-features`: exit 0.

### Prohibitions honored

- No private item widened from `pub(crate)`/private to `pub` to satisfy a link (D-05).
- No `#[allow(rustdoc::private_intra_doc_links)]` or other lint-suppression attribute
  added.
- No library source behaviour changed -- every edit is a doc comment, the new example
  binary, the README section, or a `.planning/` evidence artifact.
- The example prints no API key and hard-codes none; its header states it needs no
  provider key and no external service.

## Orchestrator — wave 3 post-merge gate: `openapi.json` regeneration

- **Trigger:** after plans 36-04 and 36-05 landed, the post-merge `make test` gate failed exactly one
  test: `paladin-web` `openapi::tests::openapi_matches_committed_baseline` (225 passed, 1 failed).
- **Cause:** `crates/paladin-web/openapi.json` is generated from the route handlers' doc comments
  (utoipa uses rustdoc text as `summary`/`description`). Plan 36-04's D-05/D-06 fixes in
  `thread_controller.rs` reworded two doc sentences (the `[`MAX_HISTORY_LIMIT`]` private-item
  de-link and the `[`resume_thread`]`-adjacent text), so the generated document drifted from the
  committed baseline by those two `description` strings.
- **Fix:** `UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline` →
  `git diff --stat crates/paladin-web/openapi.json` = `1 file changed, 2 insertions(+), 2 deletions(-)`;
  `git diff -U0 … | grep -oE '"[a-zA-Z_]+":' | sort | uniq -c` = `4 "description":` — no path, method,
  schema, or response-code key changed. Consumers checked: the `sdk-smoke` CI job (`ci.yml:1156-1201`)
  generates clients from the file and is indifferent to description text; no `docs/src` page embeds it.
- **Result:** the single test passes after regeneration (`1 passed; 0 failed`); the rest of the suite was
  already green on the failing run (3706 passed). Committed as the wave-3 post-merge fix.

## Phase Closure Map (assembled by plan 36-13, D-24)

Assembled from the per-plan Closure Tables in `36-01-SUMMARY.md` through `36-12-SUMMARY.md` —
not re-derived from the source tree. Every one of the 143 `RD-nn` rows and every one of the 64
`EX-nn` work rows from `34-AUDIT.md` §6 appears exactly once below, each with the real commit
hash from `git log` (not the plan-time placeholder text some SUMMARYs carried). Cross-check: the
143 `RD-nn` IDs form one contiguous run 1-143 with no gap and no overlap across the eight closing
commits; the 64 `EX-nn` IDs are EX-01, EX-33, EX-55, EX-121, EX-122 plus the contiguous run
EX-62-120 (59 IDs) — both counts reproduced by hand from every SUMMARY's own table, not assumed.

### RD-nn rustdoc closure table (143 rows across 8 closing commits)

**paladin-memory — `4535ca8b`**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-01 | `crates/paladin-memory/src/token_counter/mod.rs:3` | same | unresolved link (bare shorthand to a same-file `pub use` re-export) | explicit markdown link + full crate path: `` [`HeuristicTokenCounter`](crate::token_counter::heuristic::HeuristicTokenCounter) `` | `4535ca8b` |
| RD-66 | same location group (default-feature follower) | same | closed by the RD-01 fix | same | `4535ca8b` |
| RD-126 | same location group (all-features follower) | same | closed by the RD-01 fix | same | `4535ca8b` |

**paladin-ports — `20e63d9e`**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-51 | `crates/paladin-ports/src/output/structured_executor_port.rs:158` | same | private intra-doc link (`repair_prompt`) | de-linked to plain code font, reworded to name the crate-private helper | `20e63d9e` |
| RD-127 | same location group (all-features) | same | follower | closed by the RD-51 fix | `20e63d9e` |

**paladin-storage — `81d033fb`**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-46 | `crates/paladin-storage/src/waypoint/contract_tests.rs:673` | same | private intra-doc link (`muster_progress_fixture`) | de-linked to plain code font | `81d033fb` |
| RD-128 | same location group (all-features) | same | follower | closed by the RD-46 fix | `81d033fb` |

**paladin-battalion — `9994eed5` (72 IDs: RD-10..RD-45, RD-67..RD-102)**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-13..RD-25 | `crates/paladin-battalion/src/engine/mod.rs` lines 3,4,5,6(x2),10,20,25,27,31,33,34,36 (13 distinct link targets) | same | unresolved link (bare shorthand, `//!` module-doc, crate-root scope) | explicit markdown link + full `crate::`/owning-crate path | `9994eed5` |
| RD-70..RD-82 | same location groups (default-feature) | same | followers | closed by the same fixes | `9994eed5` |
| RD-36 | `crates/paladin-battalion/src/engine/mod.rs:744` | same | private intra-doc link (`graph::validate_parley_value_for_kind`) | de-linked to plain code font | `9994eed5` |
| RD-93 | same location group (all-features) | same | follower | closed by RD-36 fix | `9994eed5` |
| RD-37 | `crates/paladin-battalion/src/engine/mod.rs:1085` | same | redundant explicit link target (`WaypointPort::get`) | collapsed to bare shorthand (already `use`-imported) | `9994eed5` |
| RD-94 | same location group (all-features) | same | follower | closed by RD-37 fix | `9994eed5` |
| RD-38 | `crates/paladin-battalion/src/engine/mod.rs:1423` | same | unresolved link (`Waypoint`, not this crate's item) | explicit `paladin_core` path | `9994eed5` |
| RD-95 | same location group (all-features) | same | follower | closed by RD-38 fix | `9994eed5` |
| RD-45 | `crates/paladin-battalion/src/engine/mod.rs:2686` | same | private intra-doc link (`superstep::run_with_namespace`) | de-linked to plain code font | `9994eed5` |
| RD-102 | same location group (all-features) | same | follower | closed by RD-45 fix | `9994eed5` |
| RD-28 | `crates/paladin-battalion/src/engine/graph.rs:772` | same | private intra-doc link (`WarGraph::validate_schedulable`) | de-linked to plain code font | `9994eed5` |
| RD-85 | same location group (all-features) | same | follower | closed by RD-28 fix | `9994eed5` |
| RD-29 | `crates/paladin-battalion/src/engine/graph.rs:1375` | same | private intra-doc link (`WarGraph::validate_aegis_undeclared_nodes`) | de-linked to plain code font | `9994eed5` |
| RD-86 | same location group (all-features) | same | follower | closed by RD-29 fix | `9994eed5` |
| RD-30..RD-34 | `crates/paladin-battalion/src/engine/graph.rs` lines 2258, 2287, 2303, 2337, 2358 (5 separate `fingerprint` doc blocks) | same | private intra-doc link (`push_field`, 5 distinct instances) | de-linked to plain code font, each at its own line | `9994eed5` |
| RD-87..RD-91 | same location groups (all-features) | same | followers | closed by RD-30..RD-34 fixes | `9994eed5` |
| RD-26 | `crates/paladin-battalion/src/engine/cache_key.rs:23` | same | unresolved link (`graph_prefix`, public) | explicit `crate::` path | `9994eed5` |
| RD-83 | same location group (all-features) | same | follower | closed by RD-26 fix | `9994eed5` |
| RD-27 | same location group | same | unresolved link (`node_prefix`, public) | explicit `crate::` path | `9994eed5` |
| RD-84 | same location group (all-features) | same | follower | closed by RD-27 fix | `9994eed5` |
| RD-35 | `crates/paladin-battalion/src/engine/directive_parser.rs:47` | same | private intra-doc link (`graph::validate_parley_value_for_kind`) | de-linked to plain code font, matching RD-36's wording | `9994eed5` |
| RD-92 | same location group (all-features) | same | follower | closed by RD-35 fix | `9994eed5` |
| RD-43 | `crates/paladin-battalion/src/engine/input_mapping.rs:30` | same | redundant explicit link target (`MusterContext`) | collapsed to bare shorthand | `9994eed5` |
| RD-100 | same location group (all-features) | same | follower | closed by RD-43 fix | `9994eed5` |
| RD-44 | `crates/paladin-battalion/src/engine/input_mapping.rs:40` | same | redundant explicit link target (`ParleyResponse`) | collapsed to bare shorthand | `9994eed5` |
| RD-101 | same location group (all-features) | same | follower | closed by RD-44 fix | `9994eed5` |
| RD-10 | `crates/paladin-battalion/src/commander.rs:35` | same | private intra-doc link (`Commander::analyze_and_select`) | de-linked to plain code font | `9994eed5` |
| RD-67 | same location group (all-features) | same | follower | closed by RD-10 fix | `9994eed5` |
| RD-11 | `crates/paladin-battalion/src/commander.rs:49` | same | private intra-doc link (`Commander::analyze_and_select`) | de-linked to plain code font, matching RD-10's wording | `9994eed5` |
| RD-68 | same location group (all-features) | same | follower | closed by RD-11 fix | `9994eed5` |
| RD-12 | `crates/paladin-battalion/src/edge_evaluator.rs:3` | same | unresolved link (`EdgeCondition`, `paladin-core`'s item) | explicit `paladin_core` path | `9994eed5` |
| RD-69 | same location group (all-features) | same | follower | closed by RD-12 fix | `9994eed5` |
| RD-39 | `crates/paladin-battalion/src/llm_decision.rs:40` | same | unresolved link (`llm_error_class`, private) | de-linked to plain code font | `9994eed5` |
| RD-96 | same location group (all-features) | same | follower | closed by RD-39 fix | `9994eed5` |
| RD-40 | `crates/paladin-battalion/src/llm_failure.rs:1` | same | unresolved link (`PaladinError::LlmFailure`) | explicit `paladin_core` path | `9994eed5` |
| RD-97 | same location group (all-features) | same | follower | closed by RD-40 fix | `9994eed5` |
| RD-41 | `crates/paladin-battalion/src/llm_failure.rs:8` | same | unresolved link (`PaladinError::LlmFailure`) | explicit `paladin_core` path | `9994eed5` |
| RD-98 | same location group (all-features) | same | follower | closed by RD-41 fix | `9994eed5` |
| RD-42 | `crates/paladin-battalion/src/llm_failure.rs:38` | same | unresolved link (`PaladinError::is_retryable`) | explicit `paladin_core` path | `9994eed5` |
| RD-99 | same location group (all-features) | same | follower | closed by RD-42 fix | `9994eed5` |

All 72 IDs (RD-10..RD-45, RD-67..RD-102) closed in the single commit `9994eed5`.

**paladin-core (published as `paladin-ai-core`) — `71a47dc9` (28 IDs: RD-52..RD-65, RD-103..RD-116)**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-54 | `crates/paladin-core/src/platform/container/trace.rs:3` (`TraceEvent`) | same | unresolved link (bare shorthand, `//!` module-doc, crate-root scope) | explicit `crate::` markdown link | `71a47dc9` |
| RD-105 | same location group (all-features) | same | follower | closed by RD-54 fix | `71a47dc9` |
| RD-55 | `trace.rs:6` (`TraceRecord`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-106 | same location group (all-features) | same | follower | closed by RD-55 fix | `71a47dc9` |
| RD-56 | `trace.rs:17` (`TraceRecord`, second mention) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-107 | same location group (all-features) | same | follower | closed by RD-56 fix | `71a47dc9` |
| RD-57 | `trace.rs:21` (`TraceEvent::DeltaMerged`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-108 | same location group (all-features) | same | follower | closed by RD-57 fix | `71a47dc9` |
| RD-58 | `trace.rs:22` (`FieldChange`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-109 | same location group (all-features) | same | follower | closed by RD-58 fix | `71a47dc9` |
| RD-59 | `trace.rs:25` (`FieldChange::value`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-110 | same location group (all-features) | same | follower | closed by RD-59 fix | `71a47dc9` |
| RD-60 | `trace.rs:36` (`TraceEvent::NodeProgress`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-111 | same location group (all-features) | same | follower | closed by RD-60 fix | `71a47dc9` |
| RD-61 | `trace.rs:37` (`TraceEvent::ParleyRaised`) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-112 | same location group (all-features) | same | follower | closed by RD-61 fix | `71a47dc9` |
| RD-62 | `trace.rs:44` (`TraceEvent::NodeProgress`, second mention) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-113 | same location group (all-features) | same | follower | closed by RD-62 fix | `71a47dc9` |
| RD-63 | `trace.rs:45` (`TraceEvent::ParleyRaised`, second mention) | same | unresolved link (bare shorthand) | explicit `crate::` markdown link | `71a47dc9` |
| RD-114 | same location group (all-features) | same | follower | closed by RD-63 fix | `71a47dc9` |
| RD-52 | `crates/paladin-core/src/platform/container/directive.rs:3` (`StateNode::run`) | same | unresolved link (cross-crate item, `paladin-battalion`) | de-linked to plain code font (D-05), no dependency added | `71a47dc9` |
| RD-103 | same location group (all-features) | same | follower | closed by RD-52 fix | `71a47dc9` |
| RD-53 | `crates/paladin-core/src/platform/container/structured.rs:13` (`extract_json`) | same | unresolved link (bare shorthand, public in-crate) | explicit `crate::` markdown link | `71a47dc9` |
| RD-104 | same location group (all-features) | same | follower | closed by RD-53 fix | `71a47dc9` |
| RD-64 | `crates/paladin-core/src/platform/container/webhook.rs:19` (`WebhookDelivery`) | same | unresolved link (bare shorthand, public in-crate) | explicit `crate::` markdown link | `71a47dc9` |
| RD-115 | same location group (all-features) | same | follower | closed by RD-64 fix | `71a47dc9` |
| RD-65 | `webhook.rs:20` (`WEBHOOK_DELIVERY_SCHEMA_VERSION`) | same | unresolved link (bare shorthand, public in-crate) | explicit `crate::` markdown link | `71a47dc9` |
| RD-116 | same location group (all-features) | same | follower | closed by RD-65 fix | `71a47dc9` |

All 28 IDs (RD-52..RD-65, RD-103..RD-116) closed in the single commit `71a47dc9`.

**paladin-llm — `6280440b` (13 IDs: RD-47..RD-50, RD-117..RD-125)**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-49 | `crates/paladin-llm/src/http_status.rs:6` | same | unclosed HTML tag (`<status>`, code span crossed a doc-comment line break) | reflowed onto one line, whole span single-line | `6280440b` |
| RD-124 | same location group (all-features) | same | follower | closed by RD-49 fix | `6280440b` |
| RD-50 | `http_status.rs:7` (`<body>`) | same, folded into line 6's reflow | unclosed HTML tag | reflowed onto one line | `6280440b` |
| RD-125 | same location group (all-features) | same | follower | closed by RD-50 fix | `6280440b` |
| RD-47 | `crates/paladin-llm/src/redaction.rs:164` | same | private intra-doc link (`JWT_MIN_SEGMENT_LEN`) | de-linked to plain code font | `6280440b` |
| RD-117 | same location group (all-features) | same | follower | closed by RD-47 fix | `6280440b` |
| RD-48 | `crates/paladin-llm/src/services/commissary.rs:89` | same | private intra-doc link (`PESSIMISTIC_TOKENS_PER_1000_BYTES`) | de-linked to plain code font | `6280440b` |
| RD-118 | same location group (all-features) | same | follower | closed by RD-48 fix | `6280440b` |
| RD-119 | `crates/paladin-llm/src/compat/engine.rs:114` | same | private intra-doc link (`CompatEngine::build_request`), openai-compatible-gated | de-linked to plain code font | `6280440b` |
| RD-120 | `compat/engine.rs:201` | same | private intra-doc link (`CompatEngine::map_error`), gated | de-linked to plain code font | `6280440b` |
| RD-121 | `compat/engine.rs:1041` | same | private intra-doc link (`classify_fetch_failure`), gated | de-linked to plain code font | `6280440b` |
| RD-122 | `crates/paladin-llm/src/gemini/adapter.rs:28` | same | private intra-doc link (`GeminiResponse`), gemini-gated | de-linked to plain code font | `6280440b` |
| RD-123 | `gemini/adapter.rs:59` | same | private intra-doc link (`GeminiAdapter::map_error`), gated | de-linked to plain code font | `6280440b` |

**paladin-web — `28bfd39f` (11 IDs: RD-07..RD-09, RD-129..RD-136)**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-07 | `crates/paladin-web/src/thread_controller.rs:483` | same | private intra-doc link (`MAX_HISTORY_LIMIT`) | de-linked to plain code font | `28bfd39f` |
| RD-134 | same location group (all-features) | same | follower | closed by RD-07 fix | `28bfd39f` |
| RD-08 | `thread_controller.rs:686` | same | private intra-doc link (`map_parley_error`) | de-linked to plain code font | `28bfd39f` |
| RD-135 | same location group (all-features) | same | follower | closed by RD-08 fix | `28bfd39f` |
| RD-09 | `thread_controller.rs:757` | same | private intra-doc link (`MAX_HISTORY_LIMIT`, 2nd mention) | de-linked to plain code font, matching RD-07's wording | `28bfd39f` |
| RD-136 | same location group (all-features) | same | follower | closed by RD-09 fix | `28bfd39f` |
| RD-129 | `crates/paladin-web/src/dev_ui_controller.rs:3` | same | unresolved link (`RunInspectorPort`, public, `paladin_ports`), dev-ui-gated | explicit `paladin_ports::input::run_inspector_port` path | `28bfd39f` |
| RD-130 | `dev_ui_controller.rs:20` | same | unresolved link (`dev_ui_inspector_page`, public, same crate), gated | explicit `crate::dev_ui_controller` path | `28bfd39f` |
| RD-131 | `dev_ui_controller.rs:69` | same | private intra-doc link (`NOT_WIRED_MESSAGE`), gated | de-linked to plain code font | `28bfd39f` |
| RD-132 | `dev_ui_controller.rs:28` | same | unresolved link (`InspectorView::supersteps`, public, `paladin_ports`), gated | explicit `paladin_ports::input::run_inspector_port` path | `28bfd39f` |
| RD-133 | `dev_ui_controller.rs:131` | same | private intra-doc link (`escape_for_script`), gated | de-linked to plain code font, no algorithm detail added | `28bfd39f` |

All 24 IDs across `paladin-llm` (13) and `paladin-web` (11) closed across the two commits
`6280440b` and `28bfd39f`.

**facade `paladin-ai` (root `src/`) — `d152ba8c` (12 IDs: RD-02..RD-06, RD-137..RD-143)**

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-02 | `src/application/services/paladin/paladin_execution_service.rs:1014` | same | private intra-doc link | de-linked, plain code font | `d152ba8c` |
| RD-138 | same location group | same | all-features follower | closed by RD-02 fix | `d152ba8c` |
| RD-03 | `src/application/services/parley/adapter.rs:28` | same | private intra-doc link | de-linked, plain code font | `d152ba8c` |
| RD-139 | same location group | same | all-features follower | closed by RD-03 fix | `d152ba8c` |
| RD-04 | `src/application/services/run/worker.rs:641` | same | private intra-doc link | de-linked, plain code font | `d152ba8c` |
| RD-140 | same location group | same | all-features follower | closed by RD-04 fix | `d152ba8c` |
| RD-05 | `src/config/agent_runtime.rs:1174` | same | private intra-doc link | de-linked, plain code font | `d152ba8c` |
| RD-141 | same location group | same | all-features follower | closed by RD-05 fix | `d152ba8c` |
| RD-06 | `src/presets/mod.rs:55` | same | private intra-doc link | de-linked, plain code font | `d152ba8c` |
| RD-143 | same location group | same | all-features follower | closed by RD-06 fix | `d152ba8c` |
| RD-137 | `src/application/cli/commands/eval.rs:281` | same | private intra-doc link, all-features-only (cli gate) | de-linked, plain code font | `d152ba8c` |
| RD-142 | `src/infrastructure/telemetry/otel_sink.rs:42` | same | private intra-doc link, all-features-only (otel gate) | de-linked, plain code font | `d152ba8c` |

**Coverage check:** memory(3) + ports(2) + storage(2) + battalion(72) + core(28) + llm(13) +
web(11) + facade(12) = **143**. The eight ranges (1, 2-6, 7-9, 10-45, 46, 47-50, 51, 52-65, 66,
67-102, 103-116, 117-125, 126-128, 129-136, 137-143) tile `RD-01`..`RD-143` with no gap and no
overlap — verified by hand against each commit's own ID list above.

**ID inventory (every `RD-nn` spelled out literally, not range-compressed, so a grep for any
single ID succeeds against this file — the narrative tables above use `..`-range notation for
readability, matching every per-plan SUMMARY's own style, which a literal per-ID grep does not
match):**

| Commit | Crate | IDs |
|---|---|---|
| `4535ca8b` | paladin-memory | RD-01, RD-66, RD-126 |
| `20e63d9e` | paladin-ports | RD-51, RD-127 |
| `81d033fb` | paladin-storage | RD-46, RD-128 |
| `9994eed5` | paladin-battalion | RD-10, RD-11, RD-12, RD-13, RD-14, RD-15, RD-16, RD-17, RD-18, RD-19, RD-20, RD-21, RD-22, RD-23, RD-24, RD-25, RD-26, RD-27, RD-28, RD-29, RD-30, RD-31, RD-32, RD-33, RD-34, RD-35, RD-36, RD-37, RD-38, RD-39, RD-40, RD-41, RD-42, RD-43, RD-44, RD-45, RD-67, RD-68, RD-69, RD-70, RD-71, RD-72, RD-73, RD-74, RD-75, RD-76, RD-77, RD-78, RD-79, RD-80, RD-81, RD-82, RD-83, RD-84, RD-85, RD-86, RD-87, RD-88, RD-89, RD-90, RD-91, RD-92, RD-93, RD-94, RD-95, RD-96, RD-97, RD-98, RD-99, RD-100, RD-101, RD-102 |
| `71a47dc9` | paladin-core (published as paladin-ai-core) | RD-52, RD-53, RD-54, RD-55, RD-56, RD-57, RD-58, RD-59, RD-60, RD-61, RD-62, RD-63, RD-64, RD-65, RD-103, RD-104, RD-105, RD-106, RD-107, RD-108, RD-109, RD-110, RD-111, RD-112, RD-113, RD-114, RD-115, RD-116 |
| `6280440b` | paladin-llm | RD-47, RD-48, RD-49, RD-50, RD-117, RD-118, RD-119, RD-120, RD-121, RD-122, RD-123, RD-124, RD-125 |
| `28bfd39f` | paladin-web | RD-07, RD-08, RD-09, RD-129, RD-130, RD-131, RD-132, RD-133, RD-134, RD-135, RD-136 |
| `d152ba8c` | facade paladin-ai | RD-02, RD-03, RD-04, RD-05, RD-06, RD-137, RD-138, RD-139, RD-140, RD-141, RD-142, RD-143 |

Programmatically verified: 143 total IDs listed, 143 unique, zero missing from the 1-143 range.

### EX-nn examples-currency closure table (64 rows)

| ID | capability / finding | program or file | README section | commit |
|---|---|---|---|---|
| EX-01 | stale minimum Rust version ("1.70" -> "1.88") | `examples/README.md` Getting Started block | n/a (README text fix) | `c9129dd0` |
| EX-121 | gallery-completeness gap — 11 programs never mentioned | `examples/README.md` | 5 new `##` sections + 1 joined section | `c9129dd0` |
| EX-122 | drifted `PaladinResult` field names (`response.content`/`token_usage.total_tokens`/`execution_time` -> `output`/`usage.total_tokens`/`execution_time_ms`) | `examples/README.md` | n/a (README text fix) | `c9129dd0` |
| EX-33 | HTTP-service-host router parity (example) | `examples/http_service_host.rs` | HTTP Service Host | `3363d08d` (fix), `c9129dd0` (README section) |
| EX-55 | HTTP-service-host router parity (doc-examples snippet) | `crates/doc-examples/src/http_service_host.rs` | n/a (not an `examples/` gallery file) | `3363d08d` |
| EX-109 | `Commissary::new` post-PRIM-02 signature | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` (example), `87808a33` (README section) |
| EX-111 | Anthropic-shaped `TokenUsage` (cache-read/cache-write folded into `prompt_tokens`) | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` |
| EX-112 | `TokenCounterPort::is_exact` contrasted (heuristic vs. exact) | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` |
| EX-113 | exactness read live from the counter instance | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` |
| EX-114 | `resolve_context_window` + `ResolvedWindow` | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` |
| EX-115 | `WindowSource` + `WindowFallbackPolicy` | `examples/token_economy_commissary.rs` | Token Economy Examples | `7c222e85` |
| EX-62 | `WaypointPort` injection (`InMemoryWaypointStore`) | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-63 | `EngineConfig` naming every bounded-iteration/durability field | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-64 | `APP_ENGINE_MAX_SUPERSTEPS` in-process override | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-65 | checkpoint history read-back through the port | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-66 | `WaypointRetentionService` pruning | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-80 | `GRAPH_FINGERPRINT_VERSION` + `GraphMismatch` explanation | `examples/war_engine_configuration.rs` | WarEngine Configuration & Checkpoints | `c1e4a213` |
| EX-67 | `EdgeCondition::Custom` evaluator, fail-closed vs. registered contrast | `examples/control_flow_dynamic_routing.rs` | Control Flow & Dynamic Routing | `4752ce53` |
| EX-68 | child `WarGraph` as a `NodeSpec::Battalion` node, `StateMap` propagation + `ThreadId::child` | `examples/control_flow_dynamic_routing.rs` | Control Flow & Dynamic Routing | `4752ce53` |
| EX-69 | LLM-driven edge decision via `LlmDecisionEvaluator` / Commander `StrategySelection::Semantic` | `examples/control_flow_dynamic_routing.rs` | Control Flow & Dynamic Routing | `4752ce53` |
| EX-70 | `APP_ENGINE_MAX_MUSTER_TASKS` override + `EngineError::MusterTaskLimitExceeded` | `examples/control_flow_dynamic_routing.rs` | Control Flow & Dynamic Routing | `4752ce53` |
| EX-71 | Pause at a Gate | `examples/human_in_the_loop_gate.rs` | Human-in-the-Loop | `fdbd57a5` |
| EX-72 | Resume with typed responses (total validation) | `examples/human_in_the_loop_gate.rs` | Human-in-the-Loop | `fdbd57a5` |
| EX-73 | Replay the thread onto a new branch | `examples/human_in_the_loop_gate.rs` | Human-in-the-Loop | `fdbd57a5` |
| EX-74 | Drain in-flight work | `examples/graceful_shutdown.rs` | Graceful Shutdown | `da71e046` |
| EX-75 | Configure the grace period from the environment | `examples/graceful_shutdown.rs` | Graceful Shutdown | `da71e046` |
| EX-76 | Toggle graceful shutdown off and on | `examples/graceful_shutdown.rs` | Graceful Shutdown | `da71e046` |
| EX-83 | Custom `ExecutionMiddleware` hooks | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-84 | `AgentRuntimeConfig`-resolved built-in middleware | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-85 | Custom `TokenCounterPort` injection | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-86 | Context-window management (`HistoryTrimmer` + `SummarizationMiddleware`) | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-87 | `ConfinedVault` structural memory namespacing | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-89 | Fail-run tool error mode | `examples/agent_runtime_middleware.rs` | Agent Runtime & Middleware | `4f88bf66` |
| EX-88 | Schema-validated structured output (accept + reject) | `examples/structured_output_schema.rs` | Structured Output | `0d4f043b` |
| EX-90 | JSON Schema derivation via `schemars` | `examples/structured_output_schema.rs` | Structured Output | `0d4f043b` |
| EX-116 | `RagRetrievalResult` typed retrieval | `examples/sanctum_rag_retrieval.rs` | RAG & Retrieval | `b8a78324` |
| EX-117 | `ShedItem` shed records | `examples/sanctum_rag_retrieval.rs` | RAG & Retrieval | `b8a78324` |
| EX-118 | Typed `RagRetrievalError` | `examples/sanctum_rag_retrieval.rs` | RAG & Retrieval | `b8a78324` |
| EX-119 | `retrieve_context_with_timeout` free function | `examples/sanctum_rag_retrieval.rs` | RAG & Retrieval | `b8a78324` |
| EX-120 | Exact `TokenCounterPort` injection (`with_token_counter`) | `examples/sanctum_rag_retrieval.rs` | RAG & Retrieval | `b8a78324` |
| EX-77 | Thread state route | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-78 | Thread resume route | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-79 | Thread history route | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-91 | Run submission | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-92 | Run streaming (SSE) | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-93 | Run cancellation | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-94 | Assistants (create/publish/list versions) | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-95 | Schedules (create/list) | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-98 | Run queue backend selection | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-99 | Run store backend selection | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-104 | Dev-ui inspector route | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-110 | Token usage (prompt/completion split) | `examples/platform_api_client.rs` | Platform API | `64e597a0` |
| EX-96 | Webhook signature verification | `examples/webhook_receiver.rs` | Platform API | `1c21d61c` |
| EX-97 | Private-address SSRF override | `examples/webhook_receiver.rs` | Platform API | `1c21d61c` |
| EX-81 | Redis-backed node-result cache adapter construction | `examples/node_result_cache.rs` | Node-Result Cache | `159c849d` |
| EX-82 | Node-cache enable/disable toggle | `examples/node_result_cache.rs` | Node-Result Cache | `159c849d` |
| EX-100 | Trace record envelope + real event variant names | `examples/observability_tracing.rs` | Observability & Tracing | `be0ea584` |
| EX-101 | `TraceConfig`-driven sink composition (`build_run_sink`) | `examples/observability_tracing.rs` | Observability & Tracing | `be0ea584` |
| EX-102 | `PALADIN_TRACE_OTEL_ENABLED` environment toggle | `examples/observability_tracing.rs` | Observability & Tracing | `be0ea584` |
| EX-108 | Persisted trace history (`RunTracePort::read`) | `examples/observability_tracing.rs` | Observability & Tracing | `be0ea584` |
| EX-103 | OTLP trace export sink (`OtelTraceSink`) | `examples/observability_otel_export.rs` | Observability & Tracing | `4d449a7f` |
| EX-105 | Scenario declaration + `ScenarioRunner::run_case` | `examples/eval_scenarios_demo.rs` | Evaluation | `8168329a` |
| EX-106 | `PALADIN_EVAL_LIVE` live-mode toggle | `examples/eval_scenarios_demo.rs` | Evaluation | `8168329a` |
| EX-107 | CLI `eval-run` command-line form | `examples/eval_scenarios_demo.rs` | Evaluation | `8168329a` |

**Coverage check:** EX-01, EX-33, EX-55, EX-121, EX-122 (5 rows) plus the contiguous run
EX-62-EX-120 (59 rows, every integer present — verified by hand against every per-plan table
above) = **64**, matching `34-AUDIT.md` §6's own count and this plan's `must_haves`.

### Drift record (D-02)

Two known post-audit drift observations, both recorded with the evidence the closing plans
captured rather than silently reconciled:

1. **Zero drift at phase start (plan 36-01, `36-evidence/36-01-baseline.txt`).** The phase's
   dispatch-time HEAD re-ran `34-AUDIT.md`'s own capture commands and found the baseline
   byte-identical to the audit and to `36-RESEARCH.md`'s independent re-baseline: `cargo doc
   --workspace --no-deps` still produced 73 total `warning:` lines (65 content diagnostics
   across 8 crates + 8 per-crate summary lines) and the all-features sweep still exited 101.
   No `RD-144`+ or `EX-123`+ row was minted at phase start.
2. **RAG capability-token drift found and recorded, not fixed, by plan 36-08.** Re-running the
   five RAG capability-token greps from the Phase 34 audit against both `examples/` and
   `crates/doc-examples/src/` before writing that plan's Task 3 found that three of the five
   tokens (`RagRetrievalResult`, `ShedItem`, `retrieve_context_with_timeout`) already had
   non-zero hits in `crates/doc-examples/src/sanctum_vector_memory.rs`, a module Phase 35 added
   *after* the Phase 34 audit SHA — the audit's recorded zero-hit figure for those three tokens
   was already stale before this phase started. The gap plan 36-08's own Task 3 closes
   (`examples/` gallery coverage, not `crates/doc-examples/`) was unaffected: `examples/` itself
   showed 0 hits for all five tokens right up to that plan's own commit `b8a78324`. Full table
   with file/hit counts: `36-evidence/36-08-examples.txt`, Task 3 section.

No plan in this phase minted a new `RD-` or `EX-` row (36-12's closing measurement found no
residual diagnostic; see the Measurement record below).

### Measurement record (baseline vs. closing, D-01/D-03)

| Measurement | Baseline | Closing | Capture |
|---|---|---|---|
| Default-feature diagnostics (`cargo doc --workspace --no-deps`, ci.yml:63 form) | RED (exit 1); 73 total `warning:` lines (65 content diagnostics across 8 crates + 8 per-crate summary lines) | **GREEN (exit 0); 0 `warning:` lines** | `36-evidence/36-01-baseline.txt` (baseline) -> `36-evidence/36-12-closing-measurement.txt` (closing) |
| Workspace all-features exit code (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`) | exit 101 | **exit 0** | `36-evidence/36-01-baseline.txt` -> `36-evidence/36-12-closing-measurement.txt` |
| Per-crate all-features errors | 77 errors across 8 crates (`36-RESEARCH.md` re-baseline figure) | **0** — the workspace-level bar above exits 0, which is sufficient for every crate to be clean; plan 36-12 deliberately did not re-add a per-crate breakdown step to `make doc-check` (its own Decisions Made: "the workspace-level all-features already fails on the first red crate; per-crate enumeration is a debugging convenience, not a gating requirement") | `36-RESEARCH.md` (baseline) -> `36-evidence/36-01-percrate.txt` through `36-evidence/36-09-platform.txt` (per-plan per-crate sweeps, all exit 0) -> `36-evidence/36-12-closing-measurement.txt` (workspace-level closing bar) |
| Doctests (`cargo test --workspace --doc`) | 462 passed / 0 failed / 210 ignored | **462 passed / 0 failed / 210 ignored (identical — D-00f's guarantee held throughout)** | `36-evidence/36-01-baseline.txt` -> `36-evidence/36-12-closing-measurement.txt` |
| Example binary count | 48 of 48 (baseline gallery) | **62 of 62** — 14 new example programs added this phase (`token_economy_commissary`, `war_engine_configuration`, `control_flow_dynamic_routing`, `human_in_the_loop_gate`, `graceful_shutdown`, `agent_runtime_middleware`, `structured_output_schema`, `sanctum_rag_retrieval`, `platform_api_client`, `webhook_receiver`, `node_result_cache`, `observability_tracing`, `observability_otel_export`, `eval_scenarios_demo`); 48+14=62 | `36-RESEARCH.md` (baseline) -> `36-evidence/36-12-check-examples.txt` (closing, `make check-examples` 62/62) |
| `make api-surface` | 3959 items, clean | **3959 items, unchanged throughout every plan in this phase** | `36-evidence/36-01-baseline.txt` -> `36-evidence/36-12-closing-measurement.txt` |

### Ledger note (Task 2 performs the actual status move)

- **`WINDOWS.md` row 36** (`unmet-truth`, "cargo doc --workspace --no-deps emits 16 pre-existing
  warnings... unrelated to ACCT-04/token accounting") is closed by the whole 143-row enumeration
  above: the closing measurement shows 0 `warning:` lines under that exact command, so the
  condition the row names no longer holds. Closing evidence: the Measurement record's first row,
  `36-evidence/36-12-closing-measurement.txt`.
- **`WINDOWS.md` row 37** (`deviation`, "Pre-existing broken intra-doc link [HeuristicTokenCounter]
  ... crates/paladin-memory/src/token_counter/mod.rs:3") is `RD-01` with followers `RD-66` and
  `RD-126`, closed by plan 36-01's tracer task, commit `4535ca8b` — see the RD-nn table's
  paladin-memory group above.
