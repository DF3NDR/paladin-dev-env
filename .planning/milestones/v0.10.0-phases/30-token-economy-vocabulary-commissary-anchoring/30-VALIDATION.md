---
phase: 30
slug: token-economy-vocabulary-commissary-anchoring
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-14
validated: 2026-10-24
reconciled: "2026-10-24 by Phase 46 plan 46-03, reconcile-only (D-14/D-16): map rebuilt from the archived PLAN/SUMMARY/VERIFICATION; no test written"
---

# Phase 30 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

Reconciled 2026-10-24 by Phase 46 plan 46-03 on HEAD `f365604e`, reconcile-only: the eight real task rows seeded at
plan time were kept and their Status cells filled from `30-VERIFICATION.md` (status `passed`, 7/7), `30-UAT.md`
(10/10 passed) and a re-run on HEAD of every command that runs offline in the sandbox. The pre-existing
`nyquist_compliant: true` is kept because the rebuilt map supports it (VOCAB-01..03, -05, -06 and -07 each have a green
row; VOCAB-04's two rows are both superseded by Phase 38 building the Treasurer, and their re-run equivalents are green;
none is a gap). No test was written (D-14).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None in the conventional sense — Phase 30 is documentation-only and produces no testable Rust behavior. Verification is **build + grep** based. |
| **Config file** | `docs/book.toml` (`[output.linkcheck] warning-policy = "error"`) |
| **Quick run command** | `grep -rniE '\bQuartermaster\b' crates src` and the per-task greps in each plan's `<verify><automated>` (each <1 s) |
| **Full suite command** | `mdbook build docs/ && cargo doc -p paladin-ai-core --no-deps && cargo fmt --check && cargo check --workspace` (`-p paladin-core` at plan time is the package `paladin-ai-core` on HEAD) |
| **Estimated runtime** | `mdbook build docs/` ~30-60 s (measured green on `2d41c7cc`, "No broken links found"); `cargo doc -p paladin-core --no-deps` ~20-40 s warm; `cargo check --workspace` ~60-120 s warm |

At plan time all three mdBook binaries were installed locally at the CI-pinned versions
(`mdbook` 0.4.40, `mdbook-mermaid` 0.13.0, `mdbook-linkcheck` 0.7.7). They are **not** installed in the
reconciliation sandbox, so every `mdbook build docs/` leg below is marked "CI evidence" and cites the
`30-VERIFICATION.md` run ("No broken links found") and the `docs.yml` "Build MDBook" check. Phase 30 produced no
Rust tests; the HEAD successors of its vocabulary decisions are `tests/treasurer_vocabulary_guard.rs` (ADR-0050's
downstream guardrail) and the `commissary` module tests in `paladin-llm`.

---

## Sampling Rate

- **After every task commit:** that task's `<verify><automated>` block (grep/`ls` assertions,
  <1 s, plus `mdbook build docs/` for the three tasks that touch `docs/src/**`)
- **After every plan wave:** `mdbook build docs/` + `cargo doc -p paladin-core --no-deps` +
  the two exit-gate greps (retired-term absence; reserved-term symbol absence)
- **Before `/gsd-verify-work`:** the full suite command above must be green, plus the phase-level
  `.rs` scope guard from plan 30-03 Task 3
- **Max feedback latency:** ~60 s (a single `mdbook build docs/`); ~120 s for a full wave gate

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 30-01-01 | 01 | 1 | VOCAB-01, VOCAB-02 | T-30-01 / T-30-04 | ADR provenance is branch-qualified so history is never presented as a live in-tree decision; no credential-shaped string in new prose | build + source assertion | heading-sequence check on `.planning/decisions/0049-*.md` + `Next free ADR number: 0050` + the three `Commissary` list greps + case-equality check + `mdbook build docs/` | ✅ all targets exist | ✅ green (ADR-0049 re-run on HEAD: seven headings in order, `**Date:** 2026-09-14`, `conforms`, branch-qualified provenance path, one `Next free ADR number` line -- its value moved `0050` -> `0063` as ADRs 0052-0062 landed, the single-line invariant holds; the three `Commissary` list greps and the case-equality check re-run clean) ⚠ `mdbook build` leg is CI evidence |
| 30-01-02 | 01 | 1 | VOCAB-03 | T-30-02 | Documented API is provably a subset of the shipped crate surface (two anti-invention loops); page carries no credential-shaped string | build + source assertion | `mdbook build docs/` + `Commissary::`/`Consignment::` → `pub fn` loop + type → `pub struct/enum` loop + `rust,ignore` ×2 + mermaid ×1 + credential grep | ✅ exists on HEAD | ✅ green (re-run on HEAD: `docs/src/architecture/commissary.md` linked once from `docs/src/SUMMARY.md`; both anti-invention loops -- every `Commissary::`/`Consignment::` fn resolves to a `pub fn` in `crates/paladin-llm/src/services/commissary.rs`, 0 unresolved; `rust,ignore` x3 and `mermaid` x1 on the page now, the plan's `rust,ignore` x2 grew with the Phase 31-33 additions; `commissary` module tests 25 passed) ⚠ `mdbook build` leg is CI evidence |
| 30-02-01 | 02 | 2 | VOCAB-04 | T-30-06 / T-30-07 | Reserved term creates no in-tree code symbol; any bare-word occurrence is a `///`/`//!` line | source assertion | heading-sequence check on `0050-*.md` + `installs` + `limits.rs` + `Milestone 14` + `GarrisonTreasury` + symbol-scoped reserved-term grep + non-doc-occurrence check | ✅ exists on HEAD | ⚠ superseded: the ADR-0050 text and heading sequence still verify (seven headings, `installs`, `limits.rs`, `Milestone 14`, `GarrisonTreasury`), but the "0/0 in-tree" `Treasurer` symbol grep this row asserted was a Phase 30-time invariant that Phase 38 deliberately ended by building the Treasurer (ADR-0052; `grep` now finds `Treasurer::new` in `src/`). Successor, re-run on HEAD: `cargo test --offline --test treasurer_vocabulary_guard` -> 3 passed (`treasurer_is_a_framework_only_word` plus two positive controls), the durable form of ADR-0050's downstream guardrail |
| 30-02-02 | 02 | 2 | VOCAB-07 | T-30-05 | Supersession scope is bounded to Phases 31-33 and cannot read as a blanket API-removal licence | source assertion | heading-sequence check on `0051-*.md` + `X-03` + `00-program-overview.md` + phase numbers 31/32/33 + `v0.10.0` + `.rs`-diff-empty check | ✅ exists on HEAD | ✅ green (re-run on HEAD: ADR-0051 seven headings in order, `X-03`, `00-program-overview.md`, phases 31/32/33, `v0.10.0` present;) -- the commit-relative `.rs`-diff-empty leg was green at commit time |
| 30-02-03 | 02 | 2 | VOCAB-04, VOCAB-07 | T-30-05 | One counter, one advance — index and directory cannot disagree | CLI + source assertion | exactly-one `Next free ADR number` line reading `0052` + `0050`/`0051` index rows + three Key Decisions rows in ascending line order + three-file `ls` | ✅ both targets exist | ⚠ superseded: the plan asserted `Next free ADR number: 0052`; HEAD reads `0063` (ADRs 0052-0062 followed). Equivalent re-run on HEAD: exactly one `Next free ADR number` line; one `| 0049 |`, `| 0050 |` and `| 0051 |` index row each; `(ADR-0049)`, `(ADR-0050)`, `(ADR-0051)` linked once each in `PROJECT.md` Key Decisions in ascending line order (1898 < 1899 < 1900); the three ADR files exist |
| 30-03-01 | 03 | 3 | VOCAB-05 | T-30-08 | Every documented config key is one that exists in the tree; no fictitious unified provider key | build + source assertion | exactly-one section heading + exactly 6 pipe-lines in the region + the four real owner strings + `Garrison store cap` + `allowance` + additions-only diff + `mdbook build docs/` | ✅ file exists | ✅ green (re-run on HEAD: one `## Token Budget Terminology` heading at `configuration.md:597`; the region holds 6 pipe-lines; `garrison.max_tokens`, `rag.max_tokens`, `agent_runtime.token_budget.max_tokens`, `ANTHROPIC_MAX_TOKENS`, `Garrison store cap` and `allowance` all present) ⚠ `mdbook build` leg is CI evidence |
| 30-03-02 | 03 | 3 | VOCAB-05 | T-30-03 / T-30-09 | Rustdoc-only change: signatures byte-identical, diff contains `///` lines only; the false `total_cost()` fallback claim is corrected | build + source assertion | `Milestone 14 / FUT-08` count == 5, all on `///` lines + three signature greps + example-line grep + doc-only diff check + `cargo doc -p paladin-core --no-deps` + `cargo fmt --check` | ✅ file exists | ⚠ superseded: the five `Milestone 14 / FUT-08` "reserved" markers were a Phase 30-time state that Phase 38 replaced when the Treasurer began producing `cost_estimate` (`herald.rs:406` and `:466` now read "produced by the Treasurer"); the count is 0 on HEAD. Equivalent re-run on HEAD: the signatures this row protected are intact (`pub cost_estimate: Option<f64>`, `pub fn total_cost(&self) -> Option<f64>`, `pub fn cost_estimate(mut self, cost_estimate: f64) -> Self`, 1 hit each) and `cargo fmt --check` exits 0; the rustdoc-only commit `2fe2b0d0` is verified comment-only (see 30-03-03). `cargo doc` leg: CI evidence (`30-VERIFICATION.md` doctest 4 passed) |
| 30-03-03 | 03 | 3 | VOCAB-06 | T-30-03 / T-30-10 | Retired term absent from the compiled tree; `.planning/` history untouched; phase-wide `.rs` scope guard | CLI + source assertion | retired-term grep exits 1 + `src/lib.rs` provenance-comment window + comment-only diff + plan-final annotation window + phase-base `*.rs` diff == exactly two paths + no `Cargo.toml`/`Cargo.lock`/migration-register + `cargo fmt --check` + `cargo check --workspace` | ✅ both targets exist | ✅ green (re-run on HEAD: `grep -rniE '\bQuartermaster\b' crates src` exits 1, no match; `SirQuartermaster` still present in `.project/project-management/paladin-project-plan-final.md:1112`; ADR-0049 cited within the annotation window. Per-commit check for the three plan-30-03 commits: `04d6f12d` is Markdown-only; `2fe2b0d0` (`herald.rs`) and `5f45927a` (`src/lib.rs`) each have 0 non-comment changed lines and `rustfmt --check` on the file as committed exits 0. The phase-base `*.rs` scope leg is commit-relative and was green at 30-03 commit time) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky / caveat / superseded*

**Requirement roll-up**

| Requirement | Tasks | Automated & green | Notes |
|---|---|---|---|
| VOCAB-01 | 30-01-01 | 1/1 | rule in `PROJECT.md`, `domain-model.md`, `copilot-instructions.md`; `Commissary` in each list |
| VOCAB-02 | 30-01-01 | 1/1 | ADR-0049 re-checked on HEAD |
| VOCAB-03 | 30-01-02 | 1/1 | anti-invention loops 0 unresolved; `commissary` module tests 25 passed; mdbook leg CI evidence |
| VOCAB-04 | 30-02-01, 30-02-03 | 0/2 green as written, 2 superseded with green re-run equivalents | ADR-0050 text greps re-run clean (`installs`, `limits.rs`, `Milestone 14`, `GarrisonTreasury`, `Paymaster`, `Comptroller`, no `Epic 5`); `tests/treasurer_vocabulary_guard.rs` 3 passed is the HEAD successor of the 0/0 grep; PROMOTION / Key Decisions rows re-checked |
| VOCAB-05 | 30-03-01, 30-03-02 | 1/2 green, 1 superseded | the `max_tokens` table is green; the `cost_estimate` markers were superseded by Phase 38 |
| VOCAB-06 | 30-03-03 | 1/1 | Quartermaster grep exits 1 on HEAD |
| VOCAB-07 | 30-02-02, 30-02-03 | 1/2 green, 1 superseded | the counter-value literal advanced `0052` -> `0063` |

**Sampling continuity:** 8 tasks, 8 `<automated>` blocks — no task lacks one, so there is no run of
3 consecutive tasks without automated feedback. Two tasks carry `❌ W0` in the *File Exists* column
because their subject file is created *by that task* (an ADR, a new mdBook page); that is
artifact-creation, not a missing test harness — see Wave 0 below.

---

## Wave 0 Requirements

**Existing infrastructure covers all phase requirements.** There is no test-framework gap to close:
this phase produces no Rust behavior to unit-test, and every verification command is runnable today.

- [x] `mdbook` 0.4.40, `mdbook-mermaid` 0.13.0, `mdbook-linkcheck` 0.7.7 — installed at the exact
      CI-pinned versions; `mdbook build docs/` measured green on `2d41c7cc` before planning.
- [x] `git` access to `origin/feature/quartermaster-prompt-budgeting` — ref present
      (`2d41c7cc9027ad9a10167f35f4ac58ff895a13c4`); the 168-line historical record reads via
      `git show`. Asserted as a `<precondition>` on plan 30-01 Task 1.
- [x] `cargo doc` / `cargo fmt` / `cargo check` — standard toolchain.
- [x] Every file to be modified already exists; the four new files are authored by their own tasks.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The ADR-0049 narrative is a reconstruction rather than a copy of the abandoned-branch record | VOCAB-02 | "Reconstructed, not copied" is a judgment about prose, not a diff; a similarity gate would produce false verdicts either way. **Closed:** `30-UAT.md` test 1 = pass. | Read `.planning/decisions/0049-commissary-design-and-rename.md` beside `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`. Confirm: the new ADR uses current vocabulary (`Commissary`/`Consignment`/`dispense`), states the shipped API, and cites the old record by path rather than transcribing its sections. |
| The vocabulary-rule wording is accurate and the three lists do not contradict each other in substance | VOCAB-01 | The greps prove each list *contains* the term and is spelled consistently; only a reader can confirm the three statements of the rule agree in meaning. **Closed:** `30-UAT.md` test 2 = pass. | Read the Ubiquitous-language bullet in `.planning/PROJECT.md`, the new prose in `docs/src/architecture/domain-model.md`, and the row in `.github/copilot-instructions.md`. Confirm all three describe the same plain-vs-Medieval split. |
| The downstream guardrail is stated as a cross-repo convention, not as a local lint | VOCAB-04 | A prose-intent check; no command can distinguish "documented convention" from "promised enforcement". **Closed:** `30-UAT.md` test 4 = pass (and `tests/treasurer_vocabulary_guard.rs` now implements the guard in this repo). | Read ADR-0050's guardrail paragraph. Confirm it does not promise a CI check or lint in this repo. |
| `[edge:VOCAB-05/concurrency]` — every commit boundary left the workspace green | VOCAB-05 | Authored as a `verification: backstop` truth: it needs per-commit evidence that the final tree cannot supply. **Closed by reconciliation evidence (2026-10-24), not by a UAT pass:** the three plan-30-03 commits were checked individually with no checkout (`git show <commit>:<path> \| rustfmt --check`, `git diff`): `04d6f12d` Markdown-only; `2fe2b0d0` and `5f45927a` each change `.rs` comment lines only (0 non-comment changed lines) and are `rustfmt`-clean. Residual: a per-commit `cargo check --workspace` was not re-run (it would need each commit checked out); a comment-only diff cannot change its outcome. | The earlier plans 30-01 and 30-02 touched no `.rs` file (ADR, Markdown and index files only; `30-VERIFICATION.md` phase-wide scope guard: exactly `herald.rs` + `src/lib.rs`, both comment-only). |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — 8 of 8
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — none exist; tooling and refs verified present
- [x] No watch-mode flags
- [x] Feedback latency < 120 s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** reconciled 2026-10-24 (Phase 46 plan 46-03, D-14) -- 8 task rows (5 green, 3 superseded with a re-run equivalent), 0 gaps, 4 manual-only rows closed

---

## Validation Audit 2026-10-24

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

- **Method:** reconcile-only (D-14). The seeded rows were real and are kept; `/gsd-validate-phase` Step 4 (gap menu) and Step 5 (`gsd-nyquist-auditor`) skipped, so no test was written.
- **Compliance flag:** the pre-existing `nyquist_compliant: true` is kept -- VOCAB-01..03, -05, -06 and -07 each have a green row, VOCAB-04 is carried by two superseded rows whose re-run equivalents are green (the ADR-0050 text greps and `treasurer_vocabulary_guard`, 3 passed), no row is a gap, and the four manual-only rows cite `30-UAT.md` passes or the per-commit evidence above.
- **Re-run on HEAD 2026-10-24 (all as stated):** `grep -rniE '\bQuartermaster\b' crates src` (exit 1, none); `cargo test --offline --test treasurer_vocabulary_guard` (3 passed); `cargo test --offline -p paladin-llm --lib commissary` (25 passed); `cargo fmt --check` (exit 0); the file, heading, count and anti-invention `grep` legs named in the map.
- **CI evidence (not re-run):** every `mdbook build docs/` leg (`mdbook` is not installed in the sandbox) and `cargo doc -p paladin-ai-core --no-deps`, cited to `30-VERIFICATION.md` and the `docs.yml` check.

## Gaps

None.
