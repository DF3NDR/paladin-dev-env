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
