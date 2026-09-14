---
phase: 30
slug: token-economy-vocabulary-commissary-anchoring
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-14
---

# Phase 30 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None in the conventional sense — Phase 30 is documentation-only and produces no testable Rust behavior. Verification is **build + grep** based. |
| **Config file** | `docs/book.toml` (`[output.linkcheck] warning-policy = "error"`) |
| **Quick run command** | `grep -rniE '\bQuartermaster\b' crates src` and the per-task greps in each plan's `<verify><automated>` (each <1 s) |
| **Full suite command** | `mdbook build docs/ && cargo doc -p paladin-core --no-deps && cargo fmt --check && cargo check --workspace` |
| **Estimated runtime** | `mdbook build docs/` ~30-60 s (measured green on `2d41c7cc`, "No broken links found"); `cargo doc -p paladin-core --no-deps` ~20-40 s warm; `cargo check --workspace` ~60-120 s warm |

All three mdBook binaries are installed locally at the exact CI-pinned versions
(`mdbook` 0.4.40, `mdbook-mermaid` 0.13.0, `mdbook-linkcheck` 0.7.7), so the docs gate is a real
local command, not a CI-only fallback.

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
| 30-01-01 | 01 | 1 | VOCAB-01, VOCAB-02 | T-30-01 / T-30-04 | ADR provenance is branch-qualified so history is never presented as a live in-tree decision; no credential-shaped string in new prose | build + source assertion | heading-sequence check on `.planning/decisions/0049-*.md` + `Next free ADR number: 0050` + the three `Commissary` list greps + case-equality check + `mdbook build docs/` | ✅ all targets exist | ⬜ pending |
| 30-01-02 | 01 | 1 | VOCAB-03 | T-30-02 | Documented API is provably a subset of the shipped crate surface (two anti-invention loops); page carries no credential-shaped string | build + source assertion | `mdbook build docs/` + `Commissary::`/`Consignment::` → `pub fn` loop + type → `pub struct/enum` loop + `rust,ignore` ×2 + mermaid ×1 + credential grep | ❌ W0 — page not yet written | ⬜ pending |
| 30-02-01 | 02 | 2 | VOCAB-04 | T-30-06 / T-30-07 | Reserved term creates no in-tree code symbol; any bare-word occurrence is a `///`/`//!` line | source assertion | heading-sequence check on `0050-*.md` + `installs` + `limits.rs` + `Milestone 14` + `GarrisonTreasury` + symbol-scoped reserved-term grep + non-doc-occurrence check | ❌ W0 — ADR not yet written | ⬜ pending |
| 30-02-02 | 02 | 2 | VOCAB-07 | T-30-05 | Supersession scope is bounded to Phases 31-33 and cannot read as a blanket API-removal licence | source assertion | heading-sequence check on `0051-*.md` + `X-03` + `00-program-overview.md` + phase numbers 31/32/33 + `v0.10.0` + `.rs`-diff-empty check | ❌ W0 — ADR not yet written | ⬜ pending |
| 30-02-03 | 02 | 2 | VOCAB-04, VOCAB-07 | T-30-05 | One counter, one advance — index and directory cannot disagree | CLI + source assertion | exactly-one `Next free ADR number` line reading `0052` + `0050`/`0051` index rows + three Key Decisions rows in ascending line order + three-file `ls` | ✅ both targets exist | ⬜ pending |
| 30-03-01 | 03 | 3 | VOCAB-05 | T-30-08 | Every documented config key is one that exists in the tree; no fictitious unified provider key | build + source assertion | exactly-one section heading + exactly 6 pipe-lines in the region + the four real owner strings + `Garrison store cap` + `allowance` + additions-only diff + `mdbook build docs/` | ✅ file exists | ⬜ pending |
| 30-03-02 | 03 | 3 | VOCAB-05 | T-30-03 / T-30-09 | Rustdoc-only change: signatures byte-identical, diff contains `///` lines only; the false `total_cost()` fallback claim is corrected | build + source assertion | `Milestone 14 / FUT-08` count == 5, all on `///` lines + three signature greps + example-line grep + doc-only diff check + `cargo doc -p paladin-core --no-deps` + `cargo fmt --check` | ✅ file exists | ⬜ pending |
| 30-03-03 | 03 | 3 | VOCAB-06 | T-30-03 / T-30-10 | Retired term absent from the compiled tree; `.planning/` history untouched; phase-wide `.rs` scope guard | CLI + source assertion | retired-term grep exits 1 + `src/lib.rs` provenance-comment window + comment-only diff + plan-final annotation window + phase-base `*.rs` diff == exactly two paths + no `Cargo.toml`/`Cargo.lock`/migration-register + `cargo fmt --check` + `cargo check --workspace` | ✅ both targets exist | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

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
| The ADR-0049 narrative is a reconstruction rather than a copy of the abandoned-branch record | VOCAB-02 | "Reconstructed, not copied" is a judgment about prose, not a diff; a similarity gate would produce false verdicts either way | Read `.planning/decisions/0049-commissary-design-and-rename.md` beside `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`. Confirm: the new ADR uses current vocabulary (`Commissary`/`Consignment`/`dispense`), states the shipped API, and cites the old record by path rather than transcribing its sections. |
| The vocabulary-rule wording is accurate and the three lists do not contradict each other in substance | VOCAB-01 | The greps prove each list *contains* the term and is spelled consistently; only a reader can confirm the three statements of the rule agree in meaning | Read the Ubiquitous-language bullet in `.planning/PROJECT.md`, the new prose in `docs/src/architecture/domain-model.md`, and the row in `.github/copilot-instructions.md`. Confirm all three describe the same plain-vs-Medieval split. |
| The downstream guardrail is stated as a cross-repo convention, not as a local lint | VOCAB-04 | A prose-intent check; no command can distinguish "documented convention" from "promised enforcement" | Read ADR-0050's guardrail paragraph. Confirm it does not promise a CI check or lint in this repo. |
| `[edge:VOCAB-05/concurrency]` — every commit boundary left the workspace green | VOCAB-05 | Authored as a `verification: backstop` truth: it needs per-commit evidence that the final tree cannot supply | For each Phase 30 commit: `git stash` nothing, check out the commit, run `cargo fmt --check`. A verifier without that evidence must abstain (`human_needed`, reason `insufficient_spec`) rather than pass it. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — 8 of 8
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — none exist; tooling and refs verified present
- [x] No watch-mode flags
- [x] Feedback latency < 120 s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending — set `status: validated` at `/gsd-validate-phase` or phase close.
