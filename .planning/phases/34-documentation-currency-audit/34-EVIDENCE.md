# Phase 34 Documentation Currency Audit — Evidence Record (plan 34-01)

**Phase:** 34-documentation-currency-audit
**Branch:** `feature/phase-33`
**HEAD SHA at sweep time:** `ee1fb160f8e743e638b32beb6c4e32be4ede9325` — the Phase 34 start SHA
(D-23), recorded before this plan's first commit; every commit in this phase touches only
`.planning/`, so the source tree every row measures stays identical across every Phase 34 SHA.
**Written:** 2026-09-17

This record follows the `33-CI-EVIDENCE.md` shape: a numbered table of exact command → result →
verdict, verbatim captures referenced by number below. Its verdict column distinguishes a gate
this phase enforces from a baseline it merely carries — per D-00c, this phase's fix set is empty
by construction, so almost every row here is a **CARRIED** measurement, not a gate this plan
itself passes or fails (the one exception is the SC5 read-only proof, rows 10-11, which this
plan's own commits must satisfy).

**Toolchain block (repeated from `34-AUDIT.md`'s Measurement Header for this record's
self-containment):**

```
$ cargo --version
cargo 1.97.1 (c980f4866 2026-06-30)
$ rustc --version
rustc 1.97.1 (8bab26f4f 2026-07-14)
$ mdbook --version
mdbook v0.4.40
$ mdbook-linkcheck --version
mdbook-linkcheck 0.7.7
$ mdbook-mermaid --version
mdbook-mermaid 0.13.0
```

## Numbered command → result → verdict table

| # | Command | Result | Verdict |
|---|---------|--------|---------|
| 1 | `git rev-parse HEAD` | `ee1fb160f8e743e638b32beb6c4e32be4ede9325` | ✅ RECORDED — D-23 measured SHA |
| 2 | `cargo --version && rustc --version` (compared against `rust-toolchain.toml` `channel = "1.97.1"`) | `cargo 1.97.1 (c980f4866 2026-06-30)` / `rustc 1.97.1 (8bab26f4f 2026-07-14)` | ✅ PASS — matches pin exactly, no drift (D-12 closed per RESEARCH.md Finding F-12) |
| 3 | `mdbook --version && mdbook-linkcheck --version && mdbook-mermaid --version` (compared against `docs.yml:46,50,54`) | `mdbook v0.4.40` / `mdbook-linkcheck 0.7.7` / `mdbook-mermaid 0.13.0` | ✅ PASS — all three match `docs.yml` pins exactly |
| 4 | `find docs/src -name '*.md' \| sort \| wc -l` | `93` | ✅ RECORDED — D-05 scope; live-counted, not copied from CONTEXT.md/RESEARCH.md |
| 5 | `bash 34-signals.sh docs/src/introduction.md` (method self-test — a `pending` page) | Nine labelled class blocks printed, classes 1/2/4/5/6/7/8 = `none`, class 3 = 5 `paladin-*` hits, class 9 = `SKIPPED — 34-shipped-tokens.txt not yet written by plan 34-02`; exit 0 | ✅ PASS — script runs against an arbitrary page and degrades class 9 correctly |
| 6 | `bash 34-signals.sh docs/src/appendix/doc-coverage-report.md` (the MB-01 worked-row input) | Nine labelled class blocks printed, class 3 = 9 `paladin-*` hits (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage`); classes 1/2/4/5/6/7/8 = `none`; class 9 = SKIPPED; exit 0 | ✅ PASS — feeds the §2 MB-01 row's Findings cell |
| 7 | `cargo doc --workspace --no-deps 2>&1 \| tee 34-evidence/34-01-cargo-doc-default.txt` (the exact `ci.yml:63` "Check documentation" command, D-12) | **73** `warning:` lines (`grep -c '^warning:' 34-evidence/34-01-cargo-doc-default.txt`); includes the `HeuristicTokenCounter` unresolved-link warning at line 9 of the capture (no `-->` span — see row 9); exit 0, 32.8s wall time | ⚠️ CARRIED, pre-existing, **not a gate this plan enforces** — feeds the §2 MB-01 row and the §3 note connecting it to RD-01; matches the 73-count baseline `33-CI-EVIDENCE.md` row 26 and 34-RESEARCH.md already recorded — no drift since Phase 33 close |
| 8 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps 2>&1 \| tee 34-evidence/34-01-rustdoc-memory.txt` (D-12/D-14, per-crate sweep, `paladin-memory` only) | `error: unresolved link to \`HeuristicTokenCounter\`` (no `-->` span); `error: could not document \`paladin-memory\`` — exit 101, 1 content error | ⚠️ CARRIED, pre-existing, **not a gate this plan enforces** — feeds §3 RD-01 |
| 9 | `grep -n "HeuristicTokenCounter" crates/paladin-memory/src/token_counter/mod.rs` (P-01 grep-recovery method — no `-->` span exists for a `//!`-comment link) | `3://! [\`HeuristicTokenCounter\`] is the phase-wide default: a synchronous,` | ✅ PASS — reproduces `crates/paladin-memory/src/token_counter/mod.rs:3`, matching `.planning/WINDOWS.md` row 37 exactly; validates the P-01 method plans 34-06/34-07 depend on |
| 10 | `grep -n 'Rust 1.70' examples/README.md` and `grep -A1 '\[workspace.package\]' Cargo.toml \| grep rust-version` (D-17, the EX-01 worked row) | `examples/README.md:24: - Rust 1.70 or later` vs `Cargo.toml:18: rust-version = "1.88"` | ✅ PASS — reproduces the known MSRV mismatch (Pitfall P-04); feeds §4 EX-01 |
| 11 | `git status --porcelain -- . ':!.planning'` (SC5 proof, D-22, run before this plan's every commit) | (empty) | ✅ PASS — no file outside `.planning/` is modified, created or deleted |
| 12 | `git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- . ':!.planning'` (SC5 proof, D-22 — base is the Phase 34 start SHA, not `main`; see `34-check.sh`'s deviation comment for why `git merge-base HEAD main` as literally specified in the plan text would always be non-empty on this branch, 196 files, since `main` is merged only through Phase 26) | (empty) | ✅ PASS — no non-`.planning` diff has accumulated since Phase 34 began |
| 13 | `bash 34-check.sh --seed` | `PASS` on all five seed-mode assertions (a, b, c, d1, d2); exit 0 | ✅ PASS — the mechanical completeness/read-only gate is green |

| 14 | `awk '/^## \[0\.10\.0\]/,/^## \[0\.9\.0\]/' CHANGELOG.md \| wc -l` (D-08 precedence source 2, the primary readable source) | `427` | ✅ RECORDED — every entry in this 427-line, 6-headed-subsection block was read end-to-end and turned into a §1 row or subsumed by one |
| 15 | `grep -n '^## 9\.' MIGRATION.md` (D-08 precedence source 3, locating §9.1-§9.8) | Eight headings at lines 14, 162, 287, 311, 336, 418, 671, 677 | ✅ RECORDED — every subsection read in full per the plan's `read_first` list |
| 16 | `grep -nE '^\- \[x\] \*\*(ENG\|CF\|HITL\|FT\|RT\|PLAT\|OBS\|SHIP\|VOCAB\|ACCT\|PRIM\|COMM)-[0-9]+' .planning/REQUIREMENTS.md` (D-08 precedence source 4, the capability axis) | 47 requirement bullets across the twelve v0.10.0 prefixes | ✅ RECORDED — read for the capability axis; every ID cited in a §1 row's Req ID cell traces to one of these bullets |
| 17 | `git diff v0.9.0..HEAD -- .project/current-exports.txt \| grep -c '^+'` minus the `+++` header line | `4376` | ✅ RECORDED — matches CONTEXT.md's own figure exactly; confirms the diff was measured, not copied |
| 18 | Eighteen-plus identifier-token cross-check: `for t in WarEngine PaladinError ExecutionMiddleware GarrisonEntry PaladinResult StructuredExecutorPort TokenCounterPort StopReason TokenUsage Commissary ShedItem VaultPort RagRetrievalResult resolve_context_window WindowSource BattalionError Aegis FallbackLlmAdapter StreamingResponse ChunkMetadata LlmError NodeError; do grep -cF "$t" <diff>; done` (D-08's own fifteen-token-minimum cross-check) | 16 of 22 tokens present (1-171 hits each); 6 (`Aegis`, `FallbackLlmAdapter`, `StreamingResponse`, `ChunkMetadata`, `LlmError`, `NodeError`) show 0 hits | ✅ PASS — every 0-hit token traced to `.project/current-exports.txt`'s own documented `paladin::`-facade-only scope (`grep -c '^pub paladin::' .project/current-exports.txt` = 1095 of 7924 lines, confirmed by header read), never a D-00g tree/document disagreement; recorded verbatim in `34-AUDIT.md`'s exports-diff cross-check note |
| 19 | `grep -n 'Commissary' .github/copilot-instructions.md`; `grep -n 'Commissary' .planning/PROJECT.md`; `grep -n 'Commissary' docs/src/architecture/domain-model.md` (D-10 confirmation, line-anchored) | Line 36 (naming table); line 1324 (ubiquitous-language bullet); line 30 (domain-model table) | ✅ PASS — confirms the three D-10-named lists by direct content match |
| 20 | `grep -rlni 'medieval military' docs/src/` (D-10 fourth-list search) | `docs/src/introduction.md` (plus mentions in `commissary.md`/`overview.md`/`development-setup.md`/`contributing-legacy.md` that point at the three named lists, not independent tables) | ✅ RECORDED — `introduction.md` lines 78-91 carry a genuine fourth, 12-term partial list (no `Commissary`); recorded in `34-AUDIT.md`, not judged for currency here (§2 sweep scope) |
| 21 | `bash 34-signals.sh docs/src/architecture/commissary.md` (post-token-file write, method self-test re-run) | Class 9 now prints real `grep -nFf` hits (`# Commissary`, the `Commissary` definition line), no `SKIPPED` marker; exit 0 | ✅ PASS — confirms `34-shipped-tokens.txt`'s leading `#`-prefixed comment line is correctly ignored by `grep -nFf` (it never matches page content) and class 9's degrade path has closed as D-07 requires |
| 22 | `grep -c '^\| SS-[0-9]' 34-AUDIT.md`; `grep -vc '^#' 34-shipped-tokens.txt`; `grep -oE 'SS-[0-9]+' 34-AUDIT.md \| sort \| uniq -d`; `grep -c '^#### Phase ' 34-AUDIT.md` | `91`; `91`; (empty — no duplicates); `13` | ✅ PASS — row count matches token-line count exactly, every `SS-nn` is unique, all 13 phase tables present |
| 23 | `for t in WarEngine Commissary resolve_context_window WindowSource RagRetrievalResult TokenUsage; do grep -qxF "$t" 34-shipped-tokens.txt \|\| echo MISSING $t; done` (Rule 1 deviation — see 34-02-SUMMARY.md; the plan's own literal `grep -q "\| $t \|"` command cannot match a no-markup token file) | (empty — no `MISSING` lines) | ✅ PASS — all six required tokens present as exact lines |
| 24 | `bash 34-check.sh --seed` (post-§1-write re-run) | `PASS` on all five seed-mode assertions (a, b, c, d1, d2); exit 0 | ✅ PASS — completeness/read-only gate still green after this plan's edits |
| 25 | `git status --porcelain -- . ':!.planning'` (SC5 proof, run before this plan's commit) | (empty) | ✅ PASS — no file outside `.planning/` modified, created or deleted |

## Plan 34-03, Task 1 — build baseline, orphan check, vocabulary sweep, object-store sweep

**HEAD SHA at sweep time:** `7ed822b13fbf1ffbfcf7ef6c5f682d7e9f589cde` — Phase 34 has advanced since
plan 34-01's SHA (`ee1fb160f8e743e638b32beb6c4e32be4ede9325`) through the intervening plan 34-01/
34-02 commits, all `.planning/`-only per D-23's invariance argument; the source tree these rows
measure is unchanged from the recorded Phase 34 start SHA.

| # | Command | Result | Verdict |
|---|---------|--------|---------|
| 26 | `mdbook --version && mdbook-linkcheck --version && mdbook-mermaid --version` (D-11 precondition) | `mdbook v0.4.40` / `mdbook-linkcheck 0.7.7` / `mdbook-mermaid 0.13.0` | ✅ PASS — all three match `docs.yml` pins exactly |
| 27 | `mdbook-mermaid install docs/` then `git status --porcelain -- docs` (D-22, T-34-01) | (empty) | ✅ PASS — mermaid install did not mutate `docs/`; no `git checkout -- docs/` restoration needed |
| 28 | `mdbook build docs/` (linkcheck backend active, `warning-policy = "error"`) teed to `34-evidence/34-03-mdbook-build.txt` | exit 0, 3s wall time; `Found 1006 links (0 incomplete links)`; `No broken links found` | ⚠️ CARRIED baseline (not a gate this plan enforces) — green this run; 515 fragment-resolution WARN lines are a documented mdbook-linkcheck limitation, not failures |
| 29 | `bash scripts/check-doc-examples.sh` teed to the same evidence file | exit 0; Layer 1 "All included examples compile"; Layer 1b "README Quick Example is in sync"; Layer 2 "0 checked, 616 skipped, 0 failed" | ✅ PASS |
| 30 | `bash scripts/check-doc-config.sh` teed to the same evidence file | exit 0; "154 YAML block(s) checked, 0 failed" | ✅ PASS |
| 31 | Orphan check: extract `docs/src/SUMMARY.md` link targets, `comm -23` against `find docs/src -name '*.md' \| sort` | on-disk 93, nav-reachable 93 (incl. `SUMMARY.md` itself), `comm -23` output empty | ✅ PASS — zero orphans, confirms 34-RESEARCH.md's claim by direct measurement (D-00b) |
| 32 | `grep -rniE '\bQuartermaster\b' docs/src` (D-10) | 1 hit: `docs/src/architecture/commissary.md:7` (ADR-0049 rename-rationale pointer sentence) | ⚠️ RECORDED — literal match per D-10's must-be-empty grep; classified stale content, MB-02 minted; Phase 35 decides if the ADR-pointer sentence is an intentional exception (D-00c) |
| 33 | Phase 31 D-29 `token_count`/`TokenUsage` re-check, 10 pages: `grep -n 'token_count' <page>` and `grep -n 'TokenUsage' <page>` per page | 8/10 pages clean (TokenUsage present, no bare token_count); `domain-model.md` offending (bare `token_count: usize` at line 102, no adjacent split); `memory-management.md` clean (18 `GarrisonEntry.token_count: Option<u32>` hits, a page-confirmed separate concern from the ACCT carriers, type matches live code) | ⚠️ RECORDED — MB-03 minted for `domain-model.md`'s stale `GarrisonEntry` snippet (type mismatch + 3 missing fields incl. Phase 26 `is_summary`); `memory-management.md` recorded clean with its disposition note |
| 34 | Live `GarrisonEntry` struct read: `sed -n '55,76p' crates/paladin-core/src/platform/container/garrison.rs` (proves MB-03's live-vs-doc diff) | 7 fields: `id: Uuid`, `role: ConversationRole`, `content: String`, `timestamp: DateTime<Utc>`, `metadata: HashMap<String, Value>`, `token_count: Option<u32>`, `is_summary: bool`; `#[non_exhaustive]` | ✅ RECORDED — feeds MB-03's Note cell |
| 35 | `grep -rniE 'minio\|dl\.min\.io\|quay\.io' docs/src examples` (Folded Todos, MinIO slice) | 247 hits across 27 files, full list in `34-evidence/34-03-mdbook-build.txt` | ⚠️ RECORDED — every image-pin occurrence already `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` (9 occurrences, 6 files); zero hits name a retired Docker Hub image or `dl.min.io` |
| 36 | `grep -rniE 'dl\.min\.io' docs/src examples`; `grep -rniE '(^\|[^./])minio/minio' docs/src examples \| grep -v 'quay.io/minio/minio'` | both empty | ✅ PASS — confirms zero MB-nn items from the object-store slice; RustFS evaluation stays a pending todo, unchanged |
| 37 | `bash 34-check.sh --seed` (post-Task-1 re-run) | `PASS` on all five seed-mode assertions (a, b, c, d1, d2); exit 0 | ✅ PASS — completeness/read-only gate still green |
| 38 | `git status --porcelain -- . ':!.planning'` (SC5 proof, run before this task's commit) | (empty) | ✅ PASS — no file outside `.planning/` modified, created or deleted |

## Plan 34-03, Task 2 — 18 page verdicts (root, getting-started, architecture, api-reference)

| # | Command | Result | Verdict |
|---|---------|--------|---------|
| 39 | `bash 34-signals.sh` over all 18 target pages (SUMMARY.md, introduction.md, 3 getting-started, 6 architecture, 7 api-reference) | nine labelled class blocks per page; class 9 non-`SKIPPED` for every page (token file present since plan 34-02) | ✅ RECORDED — headline per-page results feed each row's Findings cell above |
| 40 | Live-code cross-checks: `sed -n '344,349p' crates/paladin-llm/src/services/commissary.rs` (Commissary::new arity); `grep -n 'pub struct GarrisonEntry' -A20 crates/paladin-core/.../garrison.rs`; `grep -n 'pub trait LlmPort' -A20 crates/paladin-ports/src/output/llm_port.rs`; `sed -n '340,393p' src/application/services/paladin/paladin_execution_service.rs` (PaladinExecutionService::new arity) | `Commissary::new` 4-arg, no `is_exact_counter` (matches `commissary.md`); `GarrisonEntry` 7 fields incl. `is_summary` (contradicts `domain-model.md`'s 4-field `usize` snippet); `LlmPort::generate(request: LlmRequest)` single-arg (contradicts `hexagonal-design.md`'s 2-arg sample); `PaladinExecutionService::new(llm_port, circuit_breaker, garrison, arsenal)` (contradicts `design-patterns.md`'s `herald` 4th param, matches `quickstart.md`'s call site) | ⚠️ RECORDED — four confirmed live-vs-doc mismatches, one confirmed match |
| 41 | `ls crates/` (D-03 crate-graph check, both `crate-map.md` pages) | 11 library crates + `doc-examples`: `paladin-battalion`, `paladin-content`, `paladin-core`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web` | ⚠️ RECORDED — both `crate-map.md` pages (architecture and api-reference) claim "nine" crates and omit `paladin-eval`/`paladin-herald`; the Phase 33 `mem --> llm` edge (COMM-01) is absent from both mermaid diagrams |
| 42 | D-09 row-for-row: `grep -c '^\| M-B-' MIGRATION.md`; `sed -n '/^## 9\.8/,$p' MIGRATION.md \| grep -cE '^[0-9]+\.'` | 4 §9.1 rows; 7 §9.8 steps | ✅ RECORDED — entry counts for the D-09 subsection in `34-AUDIT.md` |
| 43 | D-09 comparison: `sed -n '/^## 9\.1/,/^## 9\.2/p'` and `/^## 9\.8/,$p' MIGRATION.md` read in full against `upgrading.md`'s table (lines 19-24) and checklist (lines 26-63) | 4/4 and 7/7 entries carried, 0 contradicted, 0 omitted | ✅ PASS — `upgrading.md` settled `current`; no `MB-nn` from this comparison |
| 44 | Feature-flag currency: `grep -n '^\[features\]' -A60 Cargo.toml`; `grep -n '^\[features\]' -A15 crates/paladin-llm/Cargo.toml` compared against `api-reference/crate-map.md`, `architecture/crate-map.md`, `api-reference/feature-flags.md`, `getting-started/installation.md` | live root `[features]` has 20+ flags (`otel`, `dev-ui`, `redis-cache`, `storage-postgres`, `llm-kimi/qwen/grok/ollama/gemini/openai-compatible` among them); each of the four pages above is missing a distinct subset | ⚠️ RECORDED — feeds MB-06, MB-13, MB-14, MB-15 |
| 45 | MSRV/version currency: `grep -n 'rust-version' Cargo.toml rust-toolchain.toml`; `grep -n '^version = ' Cargo.toml` | `rust-version = "1.88"`; `version = "0.10.0"` | ⚠️ RECORDED — contradicts `installation.md` (1.85.0 / 0.5.0), `stable-api.md`/`crate-map.md` (both pages, 0.5.0), `feature-flags.md` (0.5/0.8), `quickstart.md` (0.7.0), `migration-guide.md`'s framing line |
| 46 | `bash 34-check.sh --seed` (post-Task-2 re-run, after fixing 6 duplicate-ID false positives from repeated MB-nn mentions in prose — same class of Rule 1 deviation 34-01/34-02 already hit) | `PASS` on all five seed-mode assertions (a, b, c, d1, d2); exit 0 | ✅ PASS |
| 47 | `git status --porcelain -- . ':!.planning'` (SC5 proof, run before this task's commit) | (empty) | ✅ PASS |

## Plan 34-04, Task 1 — 20 user-guides page verdicts + superstep-engine decision

| # | Command | Result | Verdict |
|---|---------|--------|---------|
| 48 | `bash 34-signals.sh` over all 20 `docs/src/user-guides/*.md` pages (teed to scratchpad, per-page class 1-9 blocks) | nine labelled class blocks per page; class 9 non-`SKIPPED` for every page | ✅ RECORDED — headline per-page results feed each row's Findings cell above |
| 49 | Superstep-engine token sweep: `grep -c 'superstep' docs/src/user-guides/*.md`; `grep -c 'WarEngine' ...`; `grep -c 'Battlefield' ...`; `grep -c 'Waypoint' ...`; `grep -n 'max_supersteps' ...`; `grep -c 'Vanguard' ...` (D-06 missing-page decision) | `superstep`: 36 hits/6 pages; `WarEngine`: 19 hits/5 pages; `Battlefield`: 23 hits/5 pages; `Waypoint`: 45 hits/5 pages; `max_supersteps`: 2 hits (1 each on `control-flow.md` line 29, `fault-tolerance.md` line 399); `Vanguard`: 1 hit (`fault-tolerance.md` line 228, an unrelated compensation-routing table cell) | ⚠️ RECORDED — every hit inspected and confirmed a passing mention inside a page about a different capability, never the engine's own mechanics as primary subject; feeds the missing-page verdict, row 94, MB-30 |
| 50 | `grep -n 'full engine guide is future documentation' docs/src/user-guides/control-flow.md`; `sed -n '525,535p' .planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` | `control-flow.md:29-30` quotes the deferral verbatim; `23-CONTEXT.md:529-531` confirms "No mdBook page for the WarEngine exists... a Phase 22 residual... belongs to a docs pass or SHIP-01 (Phase 29)" | ✅ PASS — the in-tree deferral admission and its origin, reproduced verbatim in the superstep-engine decision subsection |
| 51 | `grep -n 'Since:' docs/src/user-guides/*.md docs/src/operations/*.md` (D-06 Since-marker re-check, the 5 pages CONTEXT.md names) | 3 of the 5 named pages fall in this plan's scope: `eval-harness.md:3`, `graph-visualization.md:3`, `operations/observability.md:3`, all `**Since:** v0.10.0 (Phase 28, PRD 07)` | ✅ PASS — all 3 markers' claims checked against the tree and confirmed accurate (rows 80, 83; `observability.md` settled in Task 2) |
| 52 | Live-code cross-checks: `crates/paladin-core/src/platform/container/directive.rs:40-88` (`NextStep::Parley` rustdoc); `crates/paladin-battalion/src/engine/mod.rs:662-668` (`ParleyNotSupported` "Superseded" doc comment); `crates/paladin-core/src/platform/container/waypoint.rs:389` (`GRAPH_FINGERPRINT_VERSION = "v6"`); `git log -S GRAPH_FINGERPRINT_VERSION` (v6 bump commit) | `Parley` fully implemented since Phase 24 HITL-01, `ParleyNotSupported` explicitly "no longer reachable"; fingerprint is `v6` (Phase 26 D-29), bumped by commit `d17a505f` | ⚠️ RECORDED — feeds MB-22 (`control-flow.md`'s stale Parley description) and MB-23 (`fault-tolerance.md`'s stale `v5` claim) |
| 53 | Live-code cross-checks: `crates/paladin-core/src/platform/container/arsenal/core.rs:36-93` (`ArmamentCall`/`ArmamentResult` field lists); `src/application/services/paladin/paladin_builder.rs:859` (`with_handoffs` arity); `crates/paladin-memory/src/garrison/in_memory_garrison.rs:79` (`InMemoryGarrison::new` arity); `crates/paladin-core/src/platform/container/herald.rs:49-153` (`Herald` trait, 7 methods); `crates/paladin-memory/src/services/rag_retrieval_service.rs:179` (`RagRetrievalService` naming) | `ArmamentResult` is a 5-field, non-`#[non_exhaustive]` struct (`call_id`, `success`, `output`, `error`, `execution_time_ms`); `with_handoffs(Vec<Arc<Paladin>>)` not `with_specialist(Arc<Paladin>)`; `InMemoryGarrison::new(config: GarrisonConfig)` takes 1 required arg; `Herald` has 7 methods not 3; struct is `RagRetrievalService` (camelCase Rag) not `RAGRetrievalService` | ⚠️ RECORDED — feeds MB-19 (`arsenal-tools.md`), MB-24 (`herald-output.md`), MB-27 (`paladin-agents.md`), MB-28 (`sanctum-vector-memory.md`) |
| 54 | `src/application/services/paladin/paladin_execution_service.rs:1933-1995` (`format_retrieved_context`/`rag_omission_marker` read) | `PaladinExecutionService::format_retrieved_context(&self, results: &RagRetrievalResult)` appends `rag_omission_marker` when memories are shed for budget reasons | ⚠️ RECORDED — confirms the Phase 33 RAG truncation-marker behavior `sanctum-vector-memory.md` never documents (MB-28) |
| 55 | `git log -1 --format='%H %ad %s' --date=short -S 'with_handoffs' -- src/application/services/paladin/paladin_builder.rs`; `git log -1 ... -S 'fn finalize_stream' -- crates/paladin-core/src/platform/container/herald.rs` | `with_handoffs` introduced 2026-05-30 (`ae3cd8d5`, "epic-21"); `finalize_stream` introduced 2026-05-13 (`b83325b7`, pre-`paladin-core` extraction) | ✅ PASS — confirms both mismatches predate v0.10.0 (D-00g), no Phase 22-33 REQ-ID applies to either |
| 56 | Version-pin sweep: `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` over all 20 pages, cross-checked against `grep -n '^version = ' Cargo.toml` (`0.10.0`) | 8 pages carry a stale pin: `agent-orchestrator-bridge.md` (v0.5.0), `battalion-patterns.md`/`paladin-agents.md` (0.5.0), `content-processing.md` (v0.5.0 ×2), `maneuver-flow-dsl.md` (0.8.0 ×2), `orchestration.md` (v0.8.0); 12 pages carry no version pin at all | ⚠️ RECORDED — feeds MB-18, MB-20, MB-21, MB-25, MB-26, MB-27's version-pin findings |
| 57 | Phase 32 deleted-type re-check on the three memory pages: `grep -n 'TokenCounter\b\|TokenCounterFactory\|garrison::TokenCounter' docs/src/user-guides/{garrison-memory,memory-management,sanctum-vector-memory}.md` | zero hits on all three (each page's only `TokenCounter*` references, where present, name the live `TokenCounterPort`/`TiktokenCounter`/`HeuristicTokenCounter`, never the deleted trait/factory) | ✅ PASS — explicit deleted-type disposition recorded on all three rows (82, 86, 92) per the plan's acceptance criterion |
| 58 | `bash 34-check.sh --seed` (post-Task-1 re-run, after fixing 6 duplicate-ID false positives from cross-referencing sibling rows' MB-nn IDs in prose — same class of Rule 1 deviation 34-01/34-02/34-03 already hit) | `PASS` on all five seed-mode assertions (a, b, c, d1, d2); exit 0 | ✅ PASS |
| 59 | `git status --porcelain -- . ':!.planning'` (SC5 proof, run before this task's commit) | (empty) | ✅ PASS |

## Notes

- Rows 7-8's raw captures are teed verbatim to `34-evidence/34-01-cargo-doc-default.txt` and
  `34-evidence/34-01-rustdoc-memory.txt` respectively (D-02 discretion: raw logs live in a
  `34-evidence/` subdirectory rather than being pasted inline, since the default-feature capture
  alone is 73 warnings long).
- No command in this record touches anything outside `.planning/` or the gitignored `target/`
  directory; `cargo doc`'s output lands in `target/doc/`, already `.gitignore`d.
- Class 9 of `34-signals.sh` (rows 5-6) is expected to read SKIPPED until plan 34-02 writes
  `34-shipped-tokens.txt` — this is the documented degrade path (D-07), not a defect.
- **Rows 14-25 (plan 34-02):** `34-shipped-tokens.txt` now exists (91 lines, one per §1 `SS-nn`
  row) and class 9's degrade path closes as row 21 proves — no row here retroactively edits the
  plan 34-01 verdict on rows 1-13, which were correctly `SKIPPED` at the time they were captured.

---

*Phase: 34-documentation-currency-audit*
*Evidence recorded: 2026-09-17, plan 34-01*
