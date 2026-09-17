# Phase 34: Documentation Currency Audit - Pattern Map

**Mapped:** 2026-09-17
**Files analyzed:** 5 (all `.planning`-scoped; this phase is read-only outside `.planning/`)
**Analogs found:** 5 / 5

**Scope note:** Phase 34 creates zero source files. Every file it writes lives under
`.planning/phases/34-documentation-currency-audit/`. There is no "controller/service/component"
classification here — the roles below are planning-artifact roles (audit record, evidence log,
deferred register), and the "closest analog" is the nearest prior-phase artifact of the same kind,
not Rust source. D-22/SC5 (read-only outside `.planning/`) makes this the entire pattern space.

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|-----------------|---------------|
| `34-AUDIT.md` | audit record (per-item verdict table + work lists) | batch (one row per measured item, synthesized from many command runs) | `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-DOCS-01-VERDICTS.md` | exact — same "content not mtime" verdict discipline, same per-file signal-class row |
| `34-EVIDENCE.md` | verbatim command-capture log | batch (command → result → verdict, N numbered rows) | `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` | exact — identical "Local sweep" numbered-table + verbatim-capture shape |
| `deferred-items.md` | phase-local out-of-scope register | event-driven (append-as-discovered) | `.planning/phases/31-lossless-token-accounting/deferred-items.md` | exact — same "## Plan NN-nn" heading + prose-bullet-with-evidence-citation shape |
| `34-AUDIT.md` §"mdBook build baseline" / linkcheck section | build-log excerpt (subsection of the audit record) | batch (one build run, annotated) | `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-LINKCHECK-REPORT.md` | exact — same toolchain-header + timestamped run-log shape |
| `34-0N-PLAN.md` / `34-0N-SUMMARY.md` (plan artifacts, not this agent's output) | plan/summary | n/a | standard GSD plan/summary pattern (out of this mapper's scope — planner owns) | n/a |

## Pattern Assignments

### `34-AUDIT.md` (audit record)

**Analog:** `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-DOCS-01-VERDICTS.md`

**Header / method-statement pattern** (lines 1-13):
```markdown
# DOCS-01 Per-File Currency Verdict Record (D-09)

**The single D-09 record.** One artifact, fourteen rows, appended to by plans 16-01 through
16-05 as each sweeps its assigned files. No other file carries a DOCS-01 verdict.

## Method (read before adding or trusting a row)

A verdict is settled by **content**, never by file existence or modification time
(D-00e, Success Criterion 1). "File exists" and "file was touched recently" prove nothing —
16-RESEARCH.md's Pitfall 5 found all fourteen files exist and twelve carry recent mtimes while
still containing fabricated content (`cicd.md`'s CI sample being the worked example this plan
settles). A row's Verdict is `current` only if every applicable signal class below was actually
run against that file and found to match the live tree; a row is `updated → commit` only after
the found discrepancies were fixed and the fix re-verified (`mdbook build docs/` green).
```
Adapt directly for `34-AUDIT.md`'s opening: state it is the **single D-01 canonical file**,
restate the content-not-mtime rule (D-00b) verbatim in the same voice, and name which of the 4-5
plans append which section (mirrors "appended to by plans 16-01 through 16-05").

**Signal-class table pattern** (the "producing command per signal, not a copy of this table" rule):
```markdown
**The eight signal classes** (one producing command each, run per file `F`; every Findings cell
below names the actual command that produced its result, not a copy of this table):

| # | Signal class | Producing command | Compared against |
|---|---|---|---|
| 1 | version strings | `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+' "$F"` | root `Cargo.toml:34` (`0.8.0`) |
| 2 | dependency pins | `grep -nE '^[a-z-]+ *= *\{? *version' "$F"` | that crate's `Cargo.toml` |
| 3 | crate names | `grep -noE 'paladin-[a-z-]+' "$F" | sort -u` | `ls crates/` |
| 4 | module/source paths | `grep -noE '(crates|src)/[A-Za-z0-9_/.-]+\.rs' "$F"` | `test -f` each path |
| 5 | `make` targets | `grep -noE 'make [a-z-]+' "$F"` | `grep -oE '^[a-z-]+:' Makefile` |
| 6 | workflow/job names | `grep -noE '[a-z-]+\.yml' "$F"` + quoted `jobs:` ids | `ls .github/workflows/` |
| 7 | error types | `grep -noE '[A-Z][A-Za-z]*Error(::[A-Za-z]+)?' "$F"` | `grep -rn` in `crates/` and `src/` |
```
D-07 asks for these eight classes **plus a ninth** (the shipped-surface checklist grep, D-08).
Reuse this exact table shape — one row per signal, "Producing command" column literal and
copy-pasteable, "Compared against" column naming the live-tree source of truth — and append row 9
for the D-08 checklist hit.

**Per-row verdict format:** each `16-DOCS-01-VERDICTS.md` row cites the file, the verdict
(`current`/`stale`/etc.), and a Findings cell that **names the command actually run**, never left
blank — directly reusable for `34-AUDIT.md`'s mdBook table (D-06/D-07): columns `Page | Verdict |
Findings (cmd + result) | Cites (phase + shipped item)`.

---

### `34-EVIDENCE.md` (verbatim command-capture log)

**Analog:** `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md`

**Header pattern** (lines 1-13):
```markdown
# Phase 33 Commissary In-Tree Adoption — CI Evidence Record (plan 33-06)

**Phase:** 33-commissary-in-tree-adoption
**Branch:** `feature/phase-33` (...)
**Head SHA at sweep time:** `69500c9b51a37f11215037c49318d76ea017dab3` — the tip of
`feature/phase-33` at dispatch of this plan, carrying plans 33-01 through 33-05 ...
**Written:** 2026-09-16

This record has two parts, in the `29-CI-EVIDENCE.md` shape: a **Local sweep** (every gate this
devcontainer can run without Docker — all run and recorded below, against the actual `0.10.0`
tree) and a **CI-run table** ...
```
Adapt directly: `34-EVIDENCE.md`'s header must carry the HEAD SHA (D-23), the date, and the exact
toolchain versions (`cargo --version`, `mdbook --version`, etc. — per CONTEXT.md "Specific Ideas").

**Numbered command → result → verdict table pattern** (the core reusable shape, rows 1-26 of the
analog):
```markdown
| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` (D-19 exit grep 1) | No matches (exit 1) | ✅ PASS — F6 closed |
...
| 26 | `cargo doc --workspace --no-deps` (the exact `lint` job "Check documentation" command, zero-tolerance) | **73** `warning:` lines (up from the corpus audit §8's 72, measured 2026-09-16 same-day baseline; +1 net drift over the intervening period, not attributable to this phase — see below) | ⚠️ CARRIED, pre-existing, **not a gate** (Phase 29 §8, Phase 32 32-05 precedent) |
```
Use exactly this table shape for every D-11/D-12/D-14/D-16 command: `#`, the verbatim command
(quoting which decision mandates it, e.g. "(D-12)"), the summarized/verbatim result, and a verdict
cell that states pass/fail **and** whether it is a gate this phase enforces or merely a measured
baseline it carries forward (mirrors row 26's "CARRIED, not a gate" framing — directly applicable
since D-00c makes this phase's whole fix-set empty by construction).

**Non-locally-measurable-gate pattern** (row 32, the coverage-floor row): when a D-11/D-16 command
cannot run in this devcontainer (e.g. a Docker-gated check), record the exact probe commands tried,
their failures, and route to "CI-ATTRIBUTED" rather than silently omitting the row — reuse this
shape verbatim for any Phase 34 command that turns out to need Docker.

---

### `deferred-items.md` (phase-local out-of-scope register)

**Analog:** `.planning/phases/31-lossless-token-accounting/deferred-items.md`

**Full structure pattern** (entire file, both entries):
```markdown
# Deferred Items — Phase 31

Out-of-scope discoveries logged per the executor's scope-boundary rule (only auto-fix issues
directly caused by the current task's changes).

## Plan 31-01

- **`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under
  `cargo test --workspace --all-features`.** This is a pre-existing, structural conflict
  unrelated to token accounting: the test asserts `#[cfg(feature = "cli")]` is NOT active, but
  `--all-features` always activates the `cli` feature, so the test fails deterministically
  whenever `--all-features` is passed, regardless of any other change in the tree. Confirmed
  pre-existing: the file is untouched by this plan (`git diff --stat <base> -- tests/cli_isolation_test.rs`
  is empty) ... Left unfixed per the scope boundary — out of scope for this plan.

## Plan 31-05

- **`cargo doc --workspace --no-deps --all-features` emits 16 pre-existing warnings** (...):
  private-intra-doc-link warnings (...) and unclosed-HTML-tag warnings in `paladin-llm`. None of
  the implicated files ... is in this plan's `files_modified` list ... Confirmed pre-existing per
  the scope boundary — left unfixed. The plan's own verification criterion ... is unmet for
  reasons unrelated to ACCT-04; flagged here rather than silently passed over.
```
Directly reusable header ("Out-of-scope discoveries logged per the executor's scope-boundary
rule"), one `## Plan NN-nn` heading per contributing plan, one bold-lead-in bullet per finding with
the concrete evidence (command + result) that proves it is pre-existing/out-of-scope, ending with
an explicit disposition sentence ("left unfixed", "flagged here rather than silently passed over").
For Phase 34: use `## Plan 34-0N` headings, and per D-19 each bullet must state *why* the item is
neither `MB-nn` nor `RD-nn`/`EX-nn` (the P-06 `check-public-api-examples.sh` 101-vs-76 drift and the
P-05 stale "47 examples" CI comment from RESEARCH.md are the two known candidates already
identified). Add a one-line pointer convention back into `34-AUDIT.md` §7, since D-19 requires the
audit to point at this file, not just this file to exist standalone.

---

### `34-AUDIT.md` mdBook-build-baseline subsection (linkcheck/build log)

**Analog:** `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-LINKCHECK-REPORT.md`

**Toolchain-header + run-log pattern** (lines 1-22):
```markdown
# 16-01: Local `mdbook build docs/` — Linkcheck Report (D-10)

**Date:** 2026-08-24T12:19:20Z – 2026-08-24T12:20:33Z
**Working directory:** ... (repo root)
**Command:** `mdbook build docs/`
**Toolchain (all three at CI's exact pins, `.github/workflows/docs.yml:44-54`):**

\`\`\`
$ mdbook --version
mdbook v0.4.40
$ mdbook-mermaid --version
mdbook-mermaid 0.13.0
$ mdbook-linkcheck --version
mdbook-linkcheck 0.7.7
\`\`\`

`docs/mermaid.min.js` and `docs/mermaid-init.js` were missing before this run (both are
`.gitignore`d generated assets, `.gitignore:20-22`), so `mdbook-mermaid install docs/` was run
once, first, per the plan's read-once-if-missing rule. `docs/book.toml` was diffed immediately
after and is byte-identical to its pre-install state (`git diff --exit-code docs/book.toml`
exits 0) ...

## Run 1 — first `mdbook build docs/`, FAILED (exit 101)
...
```
Reuse directly for D-11's mdBook-build-as-baseline section inside `34-AUDIT.md` (or as an
evidence-anchor in `34-EVIDENCE.md` per D-02's discretion): timestamped run window, exact toolchain
version block, an explicit note on whether `mdbook-mermaid install docs/` mutated
`docs/mermaid*.js` (directly reusable for D-22's post-install `git status --porcelain -- docs`
clean-check), and a labeled `## Run N` per attempt if a first run is red.

## Shared Patterns

### "Content, never existence/mtime" verdict discipline
**Source:** `16-DOCS-01-VERDICTS.md` Method section (quoted above)
**Apply to:** every row of `34-AUDIT.md`'s mdBook table (D-00b, D-06) — a `current` verdict with an
empty or copy-pasted Findings cell is invalid; the cell must name the command that was actually run.

### Numbered command-evidence table with pass/fail + carried-vs-gate distinction
**Source:** `33-CI-EVIDENCE.md` rows 1-32 (see full excerpt above)
**Apply to:** `34-EVIDENCE.md`'s entire body — every D-08/D-11/D-12/D-14/D-16 command becomes one
numbered row in this exact shape.

### Phase-local deferred register with per-plan sections and explicit non-fix rationale
**Source:** `31-lossless-token-accounting/deferred-items.md` (full file, quoted above)
**Apply to:** `34-documentation-currency-audit/deferred-items.md` — reuse the `## Plan NN-nn`
heading convention and the "confirmed pre-existing / out of scope — left unfixed" closing sentence
per bullet, satisfying D-19's routing requirement.

### Toolchain-pin verification block
**Source:** `16-LINKCHECK-REPORT.md` lines 8-16 (the `$ tool --version` block against
`docs.yml:44-54`'s pins)
**Apply to:** `34-AUDIT.md`'s header (per CONTEXT.md "Specific Ideas": HEAD SHA, date, `cargo
--version`, `rustc --version`, `mdbook --version`, `mdbook-linkcheck --version`,
`mdbook-mermaid --version`, and the verbatim command list).

## No Analog Found

None — every artifact this phase writes has a direct, structurally matching prior-phase analog
(all three named in the phase-specific note: `16-DOCS-01-VERDICTS.md`, `33-CI-EVIDENCE.md`,
`31/deferred-items.md`, plus `16-LINKCHECK-REPORT.md` for the build-log subsection). This phase
introduces no new artifact shape.

## Metadata

**Analog search scope:** `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/`,
`.planning/phases/33-commissary-in-tree-adoption/`, `.planning/phases/31-lossless-token-accounting/`
— all three named explicitly in this task's phase-specific note; no broader search was needed since
CONTEXT.md/RESEARCH.md already identify the exact analogs (D-02, D-07, D-19, "Reusable Assets" in
RESEARCH.md's `code_context`).
**Files scanned:** 4 (the three analogs plus their full content read in this session)
**Pattern extraction date:** 2026-09-17
