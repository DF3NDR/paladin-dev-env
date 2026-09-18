# ADR-0033: One `cargo doc` bar — ratified, measured, and its residue

## Status

Accepted

**Date:** 2026-08-08

## Context

This ADR settles HARD-07 with three separate findings, kept visually distinct because they have
different owners and different dispositions. They are not merged into one narrative.

**Finding 1 — the bar is not contested; it was settled and not written down.**
`.project/Milestone_7-Production-Hardening/Epic_4/prd-api-stabilization-pre-release-preparation.md:76`
(§4.4.3) requires `cargo doc --workspace --no-deps` to "complete without documentation warnings."
`.project/Milestone_7-Production-Hardening/Epic_1/prd-extract-infrastructure-crates.md:244` (§4.6.4)
requires the same command to "produce zero errors and zero warnings," and `:378` (§8.9, Success
Metric 4) repeats it as a release success metric. Against these,
`.project/Milestone_8-Facade-Cleanup-Shim-Resolution/Epic_5/prd-document-facade-crate-role.md:159`
(FR-19) requires only "exit 0 (warnings acceptable; must not fail)" — a strictly weaker bar on the
identical command. `.github/workflows/ci.yml:58` runs the stricter form in the required `lint` job:
`cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt`
— quoted character for character from the workflow file. All ten library crates plus the facade
carry `#![warn(missing_docs)]` (`src/lib.rs:116` and each `crates/*/src/lib.rs`), so the zero-warning
posture is not a proposal — it is what ships today, in a job every merge must pass. This ADR
ratifies a shipped answer; it does not adjudicate an open one.

**Amendment, 2026-08-24, Phase 16 / plan 16-07:** Finding 1 above states, verbatim, "All ten
library crates plus the facade carry `#![warn(missing_docs)]` (`src/lib.rs:116` and each
`crates/*/src/lib.rs`), so the zero-warning posture is not a proposal — it is what ships today."
**That sentence was inaccurate when written.** `crates/paladin-herald/src/lib.rs:20` carried
`#![allow(missing_docs)]` — the opt-out form, not the warn form — from crate creation (the
2026-06-04 facade-cleanup reconciliation) until this plan flipped it. Nine of the ten library
crates plus the facade carried the warn form on 2026-08-08; `paladin-herald` did not. The
original sentence is retained above, unedited, rather than silently rewritten (D-00d); this note
is the correction.

Plan 16-07 flipped `crates/paladin-herald/src/lib.rs:20` to `#![warn(missing_docs)]`, matching the
other nine library crates and the facade verbatim in form (confirmed byte-identical to
`crates/paladin-storage/src/lib.rs:18`). The flip was measured twice: once during this phase's
research session (M-07, "flipping the attribute and rebuilding produced zero additional warnings,"
working tree restored so the measurement left no diff) and once by this plan's own execution
(`cargo doc -p paladin-herald --no-deps` after the flip landed: `0` warnings). Both measurements
agree — the bar is now genuinely uniform across all ten library crates and the facade, at zero
remediation cost.

**The eleventh crate, `crates/doc-examples`, carries neither `#![warn(missing_docs)]` nor
`#![allow(missing_docs)]`, and this amendment records that disposition explicitly rather than
leaving it as a silent, undocumented eleventh case.** It is `publish = false` and `doctest =
false` (`crates/doc-examples/Cargo.toml:5,9`); its entire public surface is `// ANCHOR:` regions
(`crates/doc-examples/src/lib.rs:1-6`) pulled into the mdBook guide via `{{#include}}`. The
documentation *for* those items is the guide page that includes them, not rustdoc's own generated
page — requiring `missing_docs` compliance on example fixtures would generate rustdoc prose no
reader ever visits. `crates/doc-examples` is therefore recorded **out of scope for the
`missing_docs` bar**, by disposition rather than oversight, and no attribute was added to it.

**Finding 2 — the gate's configuration and the gate's current result are two different claims, and
only the first was previously verified.** This task ran the exact CI command above against this
plan's own HEAD (commit `c048938`) and it **exits 1**:

```
cargo doc --workspace --no-deps 2>&1 | tee doc-output.txt && ! grep -q "warning:" doc-output.txt
GATE_EXIT=1
```

The per-crate summary lines, captured verbatim from that run:

```
warning: `paladin-web` (lib doc) generated 13 warnings
warning: `paladin-ai` (lib doc) generated 3 warnings
warning: `paladin-battalion` (lib doc) generated 3 warnings
warning: `paladin-herald` (lib doc) generated 1 warning
```

The tree produces **20 warnings** across four crates (13 + 3 + 3 + 1 = 20; `grep -c '^warning: '
doc-output.txt` returns `24` — the 20 individual warnings plus these four per-crate summary lines).
This reproduces exactly the figure this plan's own frontmatter recorded from an earlier measurement
at commit `11e9bdb` — the count and the crate split have not moved between that commit and this
one.

The warning classes, by crate and file:

- **`paladin-web` (13):**
  - 11 `broken_intra_doc_links` (unresolved rustdoc links) across five `//!`/`///` module doc
    comments: `crates/paladin-web/src/agent_auth.rs:7` (`` [`AuthPort`] ``);
    `crates/paladin-web/src/agent_registry.rs:5,10` (`` [`Paladin`] ``/`` [`PaladinExecutorPort`] ``,
    two links per line, four total); `crates/paladin-web/src/delivery_controller.rs:8-12`
    (`` [`deliver_content`] ``, `` [`get_delivery_status`] ``, `` [`get_delivery_stats`] ``,
    `` [`create_delivery_routes`] ``, four total); `crates/paladin-web/src/openapi.rs:5-6`
    (`` [`build_openapi`] ``, `` [`docs_router`] ``, two total). Rustdoc's own warning text carries
    no `-->` file span for these eleven (a known rustdoc behaviour for links inside multi-line `//!`
    module comments); the file:line citations above were re-derived by grepping the bracketed
    identifiers, not asserted from rustdoc's output.
  - 2 `redundant_explicit_links`: `crates/paladin-web/src/agent_controller.rs:651:45` and
    `crates/paladin-web/src/app.rs:69:22`, both with an explicit path that duplicates the label.
- **`paladin-ai` (facade, 3):** `private_intra_doc_links` — public doc comments linking to private
  items: `src/infrastructure/adapters/arsenal/mcp_streamable_http_adapter.rs:18:16`
  (`` [`BearerToken::expose_secret`] ``) and `src/infrastructure/web/agent_host.rs:216:56,268:7`
  (both `` [`build_agent`] ``).
- **`paladin-battalion` (3):** `crates/paladin-battalion/src/in_memory_registry.rs:9` — two
  `broken_intra_doc_links` (`` [`paladin-core`] ``, `` [`paladin-ports`] ``, both missing the
  backtick-quoting rustdoc needs to treat them as plain text rather than link targets); `:65:44` —
  one `invalid_html_tags` (`Arc<Paladin>` read as an unclosed HTML tag).
- **`paladin-herald` (1):** `crates/paladin-herald/src/lib.rs:14:9` — one `broken_intra_doc_links`
  (`` [`TableHerald`] ``).

**Six of the twenty are not new.** `.planning/ledgers/milestone-04-06.md:129`
(`REQ-doc-build-clean`, dated 2026-08-06) already recorded exactly this `paladin-battalion` 3 /
`paladin-ai` 3 = 6, with the same file:line citations and the same warning classes (the
`in_memory_registry.rs:65` unclosed-HTML-tag, the two missing-backtick links, and the three
`build_agent`/`BearerToken::expose_secret` private-link warnings). The other fourteen — all of
`paladin-web`'s 13 plus `paladin-herald`'s 1 — are newer than that measurement; `paladin-web` did
not exist as a crate and `paladin-herald` was not yet documented to this depth in run-3. This is a
pre-existing recorded defect that has grown, not fresh drift.

**This ADR does not claim the workspace clears this gate today, and it does not claim enforcement
is in effect rather than configured.** The bar is real and CI runs it on every push; the tree does
not currently pass it. This is recorded as a dated, counted, per-crate, `file:line`-cited residue
with a named owner: **Phase 16 / DOCS-03** already scopes settling the doc bar as its own success
criterion (`.planning/ROADMAP.md`), and clearing the warnings needs `.rs` doc-comment edits that
D-23 places outside this phase's boundary. Per `10-04-SUMMARY.md`, the checkpoint on this question
selected **`q3-ratify`**: ADR-0033 ratifies the bar and records the measured debt with a named
owner, rather than widening D-23 to clear the warnings in this phase. No separate plan was added.

**Finding 3 — DEBT-03 is already discharged, and the doctest posture is measured rather than
inferred.** `crates/paladin-ports/Cargo.toml` carries no `[lib]` section at all (confirmed by
reading the file in full this session — no `doctest` key anywhere). `git log --oneline --
crates/paladin-ports/Cargo.toml` shows `2bffe22 feat(08-03): re-enable paladin-ports doctests` as
the commit that removed it — landed in **Phase 8**, not this phase.
`.github/workflows/ci.yml:238` is a bare `cargo test --workspace --doc`, carrying no `--exclude` of
any crate; the record's `ci.yml:225` citation predating this phase was stale by both line number
(13 lines) and content, corrected at source by plan 10-01. `Makefile:120-123` (`test-doc`) is
already clean — the same bare `@$(CARGO) test --workspace --doc` form.

The one surviving residue is `Makefile:432-433`, inside the `release-check` target:
```
@echo "$(CYAN)Running doc tests (excluding paladin-ports: doctests reference root crate not yet published)...$(NC)"
@$(CARGO) test --workspace --doc --exclude paladin-ports
```
The exclusion's stated reason — that `paladin-ports`' doctests reference a root crate not yet
published — stopped being true when the crate family published at `0.1.0`, and stopped being
implemented when Phase 8 re-enabled the doctests at commit `2bffe22`. The exclusion also makes
`release-check` **weaker than CI**, which is the wrong direction for a release gate: a release
candidate could pass a doc-test sweep the push gate already runs more completely.

**The seven-crate doctest posture, in the stated order (alphabetical by crate name), each with its
`Cargo.toml:LINE`** (`grep -n doctest crates/*/Cargo.toml`, re-run this session and reproducing the
same seven):

| Crate | `Cargo.toml:LINE` | Doctests observed (this session) |
|---|---|---|
| `doc-examples` (package `paladin-doc-examples`) | `crates/doc-examples/Cargo.toml:9` | `cargo test -p paladin-doc-examples --doc` → 0 tests to run |
| `paladin-content` | `crates/paladin-content/Cargo.toml:15` | `cargo test -p paladin-content --doc` → 0 tests to run |
| `paladin-herald` | `crates/paladin-herald/Cargo.toml:15` | `cargo test -p paladin-herald --doc` → 6 tests, 0 passed, 6 ignored (code fences exist, marked `ignore`, so nothing executes) |
| `paladin-llm` | `crates/paladin-llm/Cargo.toml:15` | `cargo test -p paladin-llm --doc` → 4 tests, **4 passed** |
| `paladin-memory` | `crates/paladin-memory/Cargo.toml:15` | `cargo test -p paladin-memory --doc` → 7 tests, **7 passed**; the workspace-scoped run (`cargo test --workspace --doc`) reported 8 passed for this crate — a one-test discrepancy between crate-scoped and workspace-scoped invocation this session did not resolve; both figures are recorded rather than only the higher one |
| `paladin-notifications` | `crates/paladin-notifications/Cargo.toml:15` | `cargo test -p paladin-notifications --doc` → 0 tests to run |
| `paladin-storage` | `crates/paladin-storage/Cargo.toml:15` | `cargo test -p paladin-storage --doc` → 0 tests to run |

So: two of the seven (`paladin-llm`, `paladin-memory`) actually compile and execute doctests despite
`doctest = false`, and both fully pass. `paladin-herald` has code fences but they are marked
`ignore`, so the flag's practical effect there is indistinguishable from the flag working — except
that the fences do exist and are not exercised, which is a different fact from "no fences exist."
The other four (`doc-examples`, `paladin-content`, `paladin-notifications`, `paladin-storage`) show
zero doctests regardless of the flag, because they contain no rustdoc code fences at all. **This is
recorded as observed behaviour of this Cargo toolchain and this tree, not inferred from the manifest
flag, and not asserted as a general mechanism** — this ADR does not bisect Cargo versions to explain
why `doctest = false` fails to suppress two crates' doctests.

The positive baseline this ADR does not decide: `cargo test --workspace --doc` this session ran and
passed doctests for `paladin` (the facade, 96 passed / 17 ignored), `paladin_core` (49 passed / 38
ignored), `paladin_battalion` (28 passed / 50 ignored) and `paladin_ports` (96 passed / 94 ignored).
**`paladin-web` is not `doctest = false`-gated, but this session's run shows it has zero rustdoc
code examples to execute** (`running 0 tests`) — it is eligible to run doctests, not currently
proven to run any, which nuances any framing of it as part of a "four crates that run doctests"
baseline. This distinction — eligible-but-empty versus actually-exercised — is exactly the kind of
manifest-flag inference this ADR exists to avoid making.

The ledger row for `REQ-doc-coverage-audit` is `present, unproven` rather than `satisfied`: the
coverage claim is real for the crates whose doctests actually execute and unmeasured for those that
do not, and flipping the row to `satisfied` while leaving that number unwritten is the move this ADR
exists to prevent.

**Amendment, 2026-09-18, Phase 36.1 / plan 36.1-12 — Known crate-level suppressions.** Five
workspace crates carry eight pre-existing crate-level `#![allow(rustdoc::...)]` attributes that
sit *underneath* the zero-`warning:`-line bar this ADR ratifies — none of the diagnostics they
hide were ever counted in Finding 2's 20-warning residue or in any Phase 16/36 closure table,
because both were measured against the tree *with* these allows already in force. Phase 36's own
deferred-items register (`.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/
deferred-items.md`, plan 36-12) opened this finding and offered two dispositions; this amendment
exercises its option (b) — **kept and justified, not cleared, for v0.10.0**. Removing the eight
and fixing what surfaces is a Phase-36-sized job (Phase 36's 143 `RD-nn` rows took 13 plans) and
does not belong before the v0.10.0 tag.

Each attribute, its file:line, the lint it suppresses, and the diagnostics it hides per crate when
disabled — measured 2026-09-17 at commit `fed7b72e7d31b2e8cbca36bd66a9c53db6a920ea`, verbatim in
`.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-closing-measurement.txt`,
and reconfirmed committed and unchanged at this amendment's own re-read of each site:

| File:line | Lint suppressed | Crate | Diagnostics hidden (per-crate) |
|---|---|---|---|
| `src/lib.rs:117` | `rustdoc::broken_intra_doc_links` | `paladin-ai` (facade) | 108 (shared across the facade's three attributes below) |
| `src/lib.rs:118` | `rustdoc::redundant_explicit_links` | `paladin-ai` (facade) | — |
| `src/lib.rs:119` | `rustdoc::invalid_html_tags` | `paladin-ai` (facade) | — |
| `crates/paladin-llm/src/lib.rs:49` | `rustdoc::broken_intra_doc_links` | `paladin-llm` | 69 |
| `crates/paladin-ports/src/lib.rs:51` | `rustdoc::broken_intra_doc_links` (downgrades the crate's own `#![warn(...)]` one line above it) | `paladin-ports` | 119 (shared with the sibling attribute below) |
| `crates/paladin-ports/src/lib.rs:52` | `rustdoc::redundant_explicit_links` | `paladin-ports` | — |
| `crates/paladin-storage/src/lib.rs:19` | `rustdoc::broken_intra_doc_links` | `paladin-storage` | 10 |
| `crates/paladin-notifications/src/lib.rs:14` | `rustdoc::broken_intra_doc_links` | `paladin-notifications` | 3 |

Total: **309** content diagnostics across five crates (`paladin-ports` 119 + `paladin-ai` 108 +
`paladin-llm` 69 + `paladin-storage` 10 + `paladin-notifications` 3).

**Structural reason each is hard to clear in isolation.** Cross-crate documentation links that
resolve when a doc comment is compiled inside the facade crate — which re-exports and links
across every other crate's public surface — do not resolve when the identical doc comment is
compiled in isolation inside its own leaf crate, because the target item simply is not in scope
there. `crates/paladin-ports/src/lib.rs`'s own comment directly above its attribute already
states this reason for that crate ("Some port files contain cross-crate doc links … that resolved
in the original `paladin` crate but are unavailable in this isolated crate"); the same mechanism
is why `paladin-llm`, `paladin-storage` and `paladin-notifications` carry the identical
downgrade, and why the facade's own three attributes cover a different but related class
(`redundant_explicit_links`, `invalid_html_tags`) that surfaces only once the facade's own
higher link density is compiled.

**The bar measures the tree with these allows in force.** The zero-`warning:`-line bar this ADR
ratifies and Finding 1 quotes verbatim from `ci.yml:58` is real, and CI runs it on every push — but
a green result is not a claim that `cargo doc --workspace --no-deps` produces no
`rustdoc::broken_intra_doc_links` / `redundant_explicit_links` / `invalid_html_tags` diagnostic
anywhere in the workspace. It is a claim that no such diagnostic surfaces through the paths these
eight attributes leave open. 309 diagnostics exist beneath the bar today; the bar does not see
them because these eight lines tell rustdoc not to look.

**Owner and re-check trigger**, matching the reason text the corresponding `WINDOWS.md` ledger row
carries (minted and waived by plan 36.1-13 against this same finding): kept for v0.10.0 per this
amendment; owner **v0.11.0 rustdoc-suppressions phase or quick task**; re-check at the **v0.11.0
planning kickoff, 2026-10-16**.

**No `.rs` file was changed to produce this amendment.** All eight attributes remain exactly as
committed — confirmed by re-reading each site this session — and `cargo doc --workspace --no-deps`
was re-run immediately after writing this amendment and still emits zero `warning:` lines.

## Decision

Under the D-00b precedence order (ADR → shipped tree → `.planning/codebase/` map →
`intel/code-verification.md` → PRD → DOC → checkbox — an ADR that contradicts shipped code is an
instruction to change the code), the project's one `cargo doc` bar is **zero warnings on
`cargo doc --workspace --no-deps`**, ratified here. M8 Epic 5 FR-19's "warnings acceptable; must not
fail" is recorded **superseded by outcome** — CI already runs the stricter form in a required job,
and this ADR does not weaken it to make FR-19 true.

Three sub-decisions, matching the three findings above:

(i) **The bar is ratified**, exactly as Finding 1 states it, with no change to CI or to any crate's
`#![warn(missing_docs)]` posture.

(ii) **The measured residue is recorded as debt with a named owner, and is explicitly not claimed as
met.** Twenty warnings across four crates, dated to this session's measurement against commit
`c048938`, six of them unchanged since `milestone-04-06.md`'s 2026-08-06 measurement and fourteen
newer. **Phase 16 / DOCS-03** owns closing it, because clearing rustdoc warnings needs `.rs`
doc-comment edits that D-23 places outside this ground-truth phase's boundary.

(iii) **DEBT-03 is recorded discharged by Phase 8** (commit `2bffe22`), not by this phase; the
`release-check` residue is deleted by this plan's task 2; and the seven-crate doctest posture is
recorded — not decided — and handed to **Phase 15 / the coverage-and-CI quality gates**. The
`REQ-doc-coverage-audit` ledger row is `present, unproven`, not `satisfied`.

## Considered Options

- **Ratify the zero-warning bar and record the measured residue with a named owner** (accepted) —
  matches `10-04-SUMMARY.md`'s `q3-ratify` selection; keeps a ground-truth phase writing ground
  truth rather than a code-change phase with its own review surface, and gives the debt a citable,
  dated home instead of letting it decay into folklore.
- **Ratify the bar and clear the warnings in this phase** (rejected, per the `q3-ratify` selection
  recorded in `10-04-SUMMARY.md`) — would require `.rs` doc-comment edits across four crates, which
  D-23 places outside this phase's boundary; widening the boundary was the `q3-widen` branch and it
  was not selected.
- **Adopt FR-19's warnings-acceptable bar as the single bar** (rejected) — CI already runs the
  stricter zero-warning form in a required job; adopting the looser bar would be weakening a shipped
  gate to make a record true, which inverts D-00b's precedence rather than applying it.
- **Record the bar without stating the measured state** (rejected) — leaves the gate reading as
  green when it is red today; the next phase (or DOCS-03) would inherit a surprise instead of a
  known quantity.
- **Decide the seven crates' doctest posture in this ADR** (rejected) — it is a
  coverage-and-CI-quality-gates question that Phase 15 owns, not a ground-truth deliverable; deciding
  it here would pre-empt that phase's own scoping.
- **Wire `--exclude paladin-ports` out of `release-check` by rewording rather than deleting the
  echo** (rejected) — the surviving invocation needs no explanation once the exclusion is gone;
  `test-doc` already carries none, and adding a comment here would duplicate this ADR's own record.

## Code Locations

- `.github/workflows/ci.yml:58` — the `lint` job's zero-warning `cargo doc --workspace --no-deps`
  gate, quoted verbatim in Finding 1.
- `.github/workflows/ci.yml:238` — the bare `cargo test --workspace --doc` workspace doc-test step,
  no `--exclude` of any crate.
- `crates/paladin-ports/Cargo.toml` — no `[lib]` section; the doctests it once disabled are gone.
- `2bffe22` (`git log --oneline -- crates/paladin-ports/Cargo.toml`) — `feat(08-03): re-enable
  paladin-ports doctests`, the Phase 8 commit that discharged DEBT-03.
- `Makefile:120-123` (`test-doc`) — already the bare, uncorrected form.
- `Makefile:432-433` (`release-check`, pre-fix) — the surviving `--exclude paladin-ports` flag and
  its stale explanatory echo; executed by this plan's task 2.
- `.project/Milestone_7-Production-Hardening/Epic_4/prd-api-stabilization-pre-release-preparation.md:72,76,78`
  — §4.4.1 `#![warn(missing_docs)]`, §4.4.3 the zero-warning bar, §4.4.4 the >90% coverage target.
- `.project/Milestone_7-Production-Hardening/Epic_1/prd-extract-infrastructure-crates.md:244,378` —
  §4.6.4 and §8.9 (Success Metric 4), the same zero-warning bar.
- `.project/Milestone_8-Facade-Cleanup-Shim-Resolution/Epic_5/prd-document-facade-crate-role.md:159`
  — FR-19, the minority position, quoted verbatim in Finding 1.
- `.project/Milestone_7-Production-Hardening/Epic_4/epic-4-completion-summary.md:29-50` — the Task
  5.0 quality-gate list naming the doc command among passing gates, and the acceptance-criteria row
  recording the >=90% coverage posture Met; both are annotated by this plan's task 3.
- `crates/paladin-web/src/agent_auth.rs:7`, `agent_registry.rs:5,10`, `delivery_controller.rs:8-12`,
  `openapi.rs:5-6`, `agent_controller.rs:651:45`, `app.rs:69:22` — `paladin-web`'s 13 warnings.
- `src/infrastructure/adapters/arsenal/mcp_streamable_http_adapter.rs:18:16`,
  `src/infrastructure/web/agent_host.rs:216:56,268:7` — the facade's 3 warnings.
- `crates/paladin-battalion/src/in_memory_registry.rs:9,65:44` — `paladin-battalion`'s 3 warnings.
- `crates/paladin-herald/src/lib.rs:14:9` — `paladin-herald`'s 1 warning.
- `crates/doc-examples/Cargo.toml:9`, `crates/paladin-content/Cargo.toml:15`,
  `crates/paladin-herald/Cargo.toml:15`, `crates/paladin-llm/Cargo.toml:15`,
  `crates/paladin-memory/Cargo.toml:15`, `crates/paladin-notifications/Cargo.toml:15`,
  `crates/paladin-storage/Cargo.toml:15` — the seven doctest-flag crates, alphabetical order, as
  listed in Finding 3's table.
- `.planning/ledgers/milestone-04-06.md:129` — the `REQ-doc-build-clean` row recording six warnings
  across two crates on 2026-08-06, the pre-existing subset of today's twenty.

## Code Conformance

must change

Plan 10-06 task 2 is the executor: it deletes the `--exclude paladin-ports` flag and its adjacent
stale echo at `Makefile:432-433`. **The 20-warning rustdoc residue is not executed by this phase** —
it needs `.rs` doc-comment edits outside D-23's boundary, and Phase 16 / DOCS-03 is its named owner.

## Downstream Consumers

- **Phase 15 / the coverage-and-CI quality gates** — the seven-crate doctest posture recorded in
  Finding 3 (two crates that run and pass despite `doctest = false`, four with no code examples,
  one with ignored examples, and `paladin-web`'s eligible-but-currently-empty status), to decide
  alongside the workspace's other coverage-and-CI gates, with the four/five-crate baseline recorded
  here as its starting point.
- **Phase 16 / DOCS-03** — the measured 20-warning residue, its per-crate count and file:line
  citations, and this session's measurement date (against commit `c048938`), inherited as a known
  quantity rather than rediscovered.
- **Phase 10 / HARD-01** — the ledger rows for the documentation coverage audit
  (`REQ-doc-coverage-audit`, `present, unproven`), the workspace CI job, and the M8 final quality
  gate, written by plans 10-07, 10-08 and 10-10, all citing this ADR rather than re-deriving the
  measurement.
