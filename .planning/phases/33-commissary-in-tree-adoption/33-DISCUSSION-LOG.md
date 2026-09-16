# Phase 33: Commissary In-Tree Adoption - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-16
**Phase:** 33-commissary-in-tree-adoption
**Mode:** `--auto` — every question was resolved to the recommended option without an interactive prompt; each selection is logged as `[auto]` below.
**Areas discussed:** Crate seam, Budget-only Commissary construction, Priority derivation, Dispensing semantics for RAG, Result shape and error type, Marker emission and observability, Test strategy and exit grep, COMM-04 re-seal mechanics

---

## Crate seam — where the `dispense` call lives

| Option | Description | Selected |
|--------|-------------|----------|
| `paladin-memory` depends on `paladin-llm` (`default-features = false`) | The call stays inside `RagRetrievalService` where the success criterion names it; lateral edge with precedent (`paladin-battalion` → `paladin-llm`); no cycle; publish order already correct | ✓ |
| Move rationing into the facade application layer | Leaves `paladin-memory`'s own `retrieve_context` either unbounded or silently truncating | |
| Move `Commissary` into `paladin-ports` / `paladin-core` | Rejected by Phase 32 (resolver locked in `paladin-llm`); a second §9.2 break nobody asked for | |

**Auto selection:** `[auto] Crate seam — Q: "Where does the dispense call live?" → Selected: "paladin-memory depends on paladin-llm (default-features = false)" (recommended default)`
**Notes:** `crates/paladin-battalion/Cargo.toml:67-69` is the copy template, including its "no cycle" comment. Crate-map mermaid and `paladin-memory` crate narrative follow in the same commit.

---

## Budget-only Commissary construction

| Option | Description | Selected |
|--------|-------------|----------|
| `Commissary::new` over synthetic `ProviderCapabilities`, built per call | `max_context_tokens: Some(rag.max_tokens)`, provider label `"rag"`, `reserved_completion_tokens: 0`; no Commissary API change; `RagRetrievalService::new` stays infallible | ✓ |
| New `Commissary::with_budget` constructor | Additive public API on `paladin-llm`; Phase 32 PRD §4 "no Commissary drift" stance carried forward | |
| Build once in `RagRetrievalService::new`, make `new` fallible | Extra signature break for a construction that only fails on an already-invalid config | |

| Option | Description | Selected |
|--------|-------------|----------|
| Default `HeuristicTokenCounter` + `with_token_counter` builder | Same shape as `PaladinExecutionService::with_token_counter` (Phase 26 D-13); no constructor break | ✓ |
| Fourth constructor argument | Breaking; forces every caller to choose a counter | |

**Auto selection:** `[auto] Budget-only construction — Q: "How is a Commissary built from rag.max_tokens?" → Selected: "Commissary::new over synthetic capabilities, per call" (recommended default)`; `[auto] Budget-only construction — Q: "How does the service get a TokenCounterPort?" → Selected: "Default heuristic + with_token_counter builder" (recommended default)`
**Notes:** `usize → u32` conversion errors on overflow, never clamps.

---

## Priority derivation

| Option | Description | Selected |
|--------|-------------|----------|
| Rank order (index after `rank_by_relevance`, saturating to `u8::MAX`) | Structural guarantee that the highest-scoring memories are retained; ties keep insertion order (stable sort); `top_k ≤ 100` so saturation is defensive | ✓ |
| Scaled `(1 - score) * 255` | Collapses close scores into ties; loses ordering | |
| Bucketed score bands | Arbitrary thresholds; same tie problem | |

**Auto selection:** `[auto] Priority derivation — Q: "Score → priority?" → Selected: "Rank order; label = memory UUID" (recommended default)`
**Notes:** Label is the memory UUID, never content, so `ShedItem` records and logs carry no memory text.

---

## Dispensing semantics for RAG

| Option | Description | Selected |
|--------|-------------|----------|
| Commissary default semantics (equal-share clamp, per-item marker) | PRD R1 says "return the `Stockpile`"; no Commissary behaviour change; oversized single memory is retained cut+marked rather than dropped | ✓ |
| Add a whole-item-only mode to `CommissaryPlan` | Commissary behaviour knob; `constructible_struct_adds_field` break; deferred | |
| Emulate whole-item via `per_item_min_bytes = longest body` | Hack; breaks the single-oversized-memory case (would exceed budget) | |

| Option | Description | Selected |
|--------|-------------|----------|
| Commissary default ratio `358` tokens/1000 bytes, `fixed = ""` | Conservative; `≤ budget` holds under exact counters too; header not budgeted (parity with today) | ✓ |
| Match the heuristic counter at `250` | Fills the nominal budget under the heuristic but can overshoot under exact tokenizers on dense text | |

**Auto selection:** `[auto] Dispensing semantics — Q: "Per-item share or whole-item only?" → Selected: "Commissary default semantics, no new knob" (recommended default)`; `[auto] Dispensing semantics — Q: "Planning ratio / fixed material?" → Selected: "Commissary default 358; fixed = empty" (recommended default)`
**Notes:** The ~30 % lower planned byte volume vs. the old `len / 4` estimate is a documented CHANGELOG behavioural note; the Stockpile accounting is carried on the result so callers can see real usage.

---

## Result shape and error type

| Option | Description | Selected |
|--------|-------------|----------|
| New result struct + new `paladin-memory` error enum (clean break, ADR-0051) | Retained memories with post-dispense body + `truncated`, `shed: Vec<ShedItem>`, Stockpile accounting; error wraps `SanctumError` + `CommissaryError`; §9.2 rows | ✓ |
| Keep `Vec<SanctumSearchResult>` and add a side-channel accessor | Hidden state on a shared service; renderers would print un-cut `memory.content` | |
| Add a second method, keep the old one | The old path would still truncate silently or drop the shed record — violates COMM-03 | |

**Auto selection:** `[auto] Result shape — Q: "How is ShedItem surfaced?" → Selected: "New result struct + new error enum, clean break" (recommended default)`
**Notes:** `SanctumError` (a port type) is not extended. Rationing is factored into a sync seam so the property test needs no async runtime.

---

## Marker emission and observability

| Option | Description | Selected |
|--------|-------------|----------|
| Trailing omission line in both renderers + one counts-only `info!` line; no TraceEvent | `format_for_prompt` and the facade's `format_retrieved_context` both emit it from one shared helper; Commissary's per-item marker stays inside cut bodies | ✓ |
| Only the `paladin-memory` renderer | The facade has its own formatter and is the production prompt path — it would still be silent | |
| Add a `TraceEvent` variant | New observability capability; Phase 28 owns the enum; deferred | |

**Auto selection:** `[auto] Marker emission — Q: "Where is the marker emitted?" → Selected: "Both renderers + counts-only log; no TraceEvent" (recommended default)`

---

## Test strategy and exit grep

| Option | Description | Selected |
|--------|-------------|----------|
| `proptest` unit test over the sync seam + ungated facade integration test over `InMemorySanctum` | Runs in CI without Docker; the ungated test is the F4 evidence; `proptest` already pinned in the facade | ✓ |
| Extend the `qdrant`-gated `rag_integration_tests.rs` only | Needs Qdrant/Docker, absent in this devcontainer; would not run by default | |
| Property test in the facade `tests/` | Farther from the seam; would have to go through the async path | |

**Auto selection:** `[auto] Test strategy — Q: "Where do the tests live?" → Selected: "proptest unit over a sync ration() seam + ungated InMemorySanctum integration test" (recommended default)`
**Notes:** Exit grep: `truncate_to_token_budget` and `.len() / 4` in `paladin-memory` → empty. The sweep found every other truncation site already marked (Conclave `... [truncated]`, redact/trace `[truncated]`) or char-length by design (content summariser).

---

## COMM-04 re-seal mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| Append `## 11` to the corpus audit + phase-local `33-CI-EVIDENCE.md` + pointer note | Matches "appended to the Phase 29 acceptance audit rather than a new audit"; Phase 29 D-10 (audit lives in corpus), D-12/D-17 (findings recorded, human-only sign-off) | ✓ |
| New audit document | Explicitly excluded by COMM-04 | |
| Append rows to `29-CI-EVIDENCE.md` only | Mixes two phases' evidence in one record; loses the head-SHA framing | |

**Auto selection:** `[auto] COMM-04 — Q: "Where does re-seal evidence go?" → Selected: "Append §11 to the corpus audit + 33-CI-EVIDENCE.md + pointer note" (recommended default)`
**Notes:** Gates are the Phase 29 commands re-run on the final commit; `publish-dry-run` CI job is `main`-push only so the local run is pre-merge evidence (Phase 29 D-21 two-SHA rule); `cargo doc` warnings recorded, not a gate.

---

## Claude's Discretion

- Names of the result struct, retained-memory item type, error enum and sync seam.
- Exact marker and log wording (must carry count and budget).
- Whether per-call Commissary construction is wrapped in a private helper.
- Plan/wave granularity (four natural waves sketched in CONTEXT.md).
- Whether to also extend the `qdrant`-gated RAG test with a marker assertion.

## Deferred Ideas

- Whole-item-only dispensing mode on `CommissaryPlan`.
- Per-model context-window override table for `Commissary`.
- Trace/herald surfacing of shed memories.
- Facade auto-propagating its `TokenCounterPort` into `RagRetrievalService`.
- Migrating Conclave's `truncate_output` onto the Commissary.
- Ratio tuning by counter exactness.
- Treasurer (Milestone 14).
- Reviewed, not folded: RustFS evaluation todo (out of scope).
