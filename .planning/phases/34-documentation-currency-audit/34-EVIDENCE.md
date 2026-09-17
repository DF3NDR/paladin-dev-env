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
