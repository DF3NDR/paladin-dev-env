# Phase 34: Documentation Currency Audit - Context

**Gathered:** 2026-09-17
**Status:** Ready for planning
**Mode:** `--auto` — every decision below is the recommended default, auto-selected without a user
prompt; the alternatives considered are in `34-DISCUSSION-LOG.md`. The maintainer can overrule any
D-nn by editing this file before `/gsd-plan-phase 34` runs.

<domain>
## Phase Boundary

Phase 34 delivers **one read-only, classified gap inventory** of the three documentation surfaces
— the mdBook under `docs/src/` (93 pages), the rustdoc corpus of the eleven library crates plus the
facade, and the example programs (48 files under `examples/`, the single crate-level
`crates/paladin-llm/examples/live_vendor_smoke.rs`, and the 11 modules of `crates/doc-examples`) —
measured against everything Phases 22-33 shipped, plus any v0.9.0-era gap the Phase 16 currency
pass and the Phase 28-17 / 29-06 docs plans left open. Every finding is classified as exactly one
of *missing page*, *stale content*, *rustdoc warning or broken intra-doc link*, or
*non-compiling / obsolete example*, and the inventory is partitioned into the Phase 35 (mdBook)
and Phase 36 (rustdoc + examples) work lists with each item sized.

**Not in this phase:** changing any file outside `.planning/`. No page is corrected, no rustdoc
fixed, no example rewritten. Anything found that is neither documentation nor an example goes to
the phase's deferred register, never into Phase 35 or 36. The phase's commits touch only
`.planning/` (ROADMAP Success Criterion 5).

</domain>

<decisions>
## Implementation Decisions

### Inherited — locked by earlier phases, not re-litigated
- **D-00a:** The `cargo doc` bar is **zero `warning:` lines** on `cargo doc --workspace --no-deps`,
  ratified by `.planning/decisions/0033-cargo-doc-warning-bar.md` and already enforced by the
  required `lint` job at `.github/workflows/ci.yml:62-63`. This phase measures against that bar;
  it does not reopen it.
- **D-00b:** A currency verdict is settled by **content, never by file existence or modification
  time** (Phase 16 D-00e; `16-DOCS-01-VERDICTS.md` Method). `current` means *checked and found to
  match*, never *not looked at*; a `current` row with an empty findings cell is invalid.
- **D-00c:** Audit findings are **recorded, never fixed silently** (Phase 29 D-12). For this phase
  the fix set is empty by construction — see D-19.
- **D-00d:** `.planning/WINDOWS.md` rows are never deleted, only moved to `fixed` / `waived` with a
  per-row reason (Phase 29 D-24). Rows 36 and 37 (rustdoc) stay `open` through this phase; Phase 36
  closes them.
- **D-00e:** The `# Examples` (plural) heading rule applies only to the 76 enumerated public-API
  entry points in `16-DOCS-03-ENTRY-POINTS.md`, enforced by `scripts/check-public-api-examples.sh`
  (Phase 16 D-05/D-06). The audit reports on that set only; it does not extend the rule.
- **D-00f:** The vocabulary rule is Phase 30 D-01 (units and technical ports keep industry names;
  domain roles use the Medieval-military term), `Quartermaster` is purged (Phase 30 D-14 exit grep),
  and a bare token total beside a full `TokenUsage` is governed by Phase 31 D-08 (the bare-count
  rule) with the doc hit list in Phase 31 D-29.
- **D-00g:** Precedence when a doc and the tree disagree: **shipped tree outranks any document**
  (REQUIREMENTS.md scope-time conflict record; Phase 16 D-00b). A page is stale when it contradicts
  the tree, never the reverse.

### Inventory shape and item identity
- **D-01:** The canonical inventory is **one file, `34-AUDIT.md`**, in the phase directory. It holds,
  in this order: (1) the measurement header (HEAD SHA, date, toolchain versions, exact commands);
  (2) the mdBook verdict table — one row per page; (3) the rustdoc findings table — one row per
  `warning:` line and per unresolved intra-doc link; (4) the examples table — one row per program
  and per `doc-examples` module; (5) the **Phase 35 work list**; (6) the **Phase 36 work list**;
  (7) the deferred-routing list (pointers into `deferred-items.md`). Success Criteria 1-4 each
  name "a single audit document" or "the inventory"; one file keeps those reads literally true.
  — **Reversibility:** reversible — a later split into per-partition files is a mechanical move.
- **D-02:** Verbatim tool output (the `tee`'d `cargo doc` capture, the four `cargo build` logs, the
  `mdbook build` + linkcheck log, `check-doc-examples.sh` output) lives in a companion
  **`34-EVIDENCE.md`**, following the `NN-CI-EVIDENCE.md` house pattern (Phases 28, 29, 33). The
  audit tables cite evidence by section anchor; they do not paste raw logs.
- **D-03:** Every work-list item carries a **stable ID**: `MB-nn` (mdBook, Phase 35), `RD-nn`
  (rustdoc, Phase 36), `EX-nn` (examples, Phase 36), numbered in table order and never renumbered.
  Phases 35 and 36 close items by ID, so their SUMMARY/VERIFICATION can say "MB-07 closed by
  commit X". — **Reversibility:** costly — Phases 35/36 plans and verifications will cite these
  IDs; renumbering after planning breaks those citations.
- **D-04:** Sizing is **S / M / L by edit kind**, not hours: **S** = a one-line or one-link fix on
  an existing page, a single rustdoc link repair, an example that needs one rename; **M** = a
  section rewrite or a new section on an existing page, a rustdoc block on an undocumented item
  family, an example needing a signature-level update; **L** = a new page, a new example, or a
  rewrite of more than half of an existing page or example. Each Phase 35/36 work-list row carries
  ID, classification, size, the page/file:line, and the citing phase + shipped item.

### mdBook verdict method
- **D-05:** **Scope is every `.md` under `docs/src/`** — all 93 files, including `appendix/` and
  any file not linked from `docs/src/SUMMARY.md`. An orphan (present on disk, absent from
  `SUMMARY.md`) is itself a finding, classified *stale content* with the note "orphaned from nav",
  routed to Phase 35 to link or archive.
- **D-06:** A verdict is one of `current` / `stale` / `missing`. **`missing`** is a Phase 22-33
  capability with **no page of its own in `SUMMARY.md` and no section on any existing page** that
  describes it; it produces a *missing page* row with the nav position the page should take.
  A capability that has a page but whose page omits or misdescribes a shipped item is **`stale`**
  (*stale content*), never `missing`. Every `stale` or `missing` verdict cites **the phase number
  and the shipped item** (type, route, config key, CLI subcommand, feature flag, or vocabulary
  term) the page fails to describe — Success Criterion 1 is literal about this.
- **D-07:** Evidence per page is the **Phase 16 eight signal classes** (`16-DOCS-01-VERDICTS.md`
  Method: version strings, dependency pins, crate names, source paths, `make` targets,
  workflow/job names, error types, feature flags) **plus** a ninth class for this phase: the
  **shipped-surface checklist** (D-08) grepped against the page. Each row's findings cell names the
  command actually run per class, or "checked, matches" — never blank. Git log and mtime are
  context, never a verdict (D-00b).
- **D-08:** The **shipped-surface checklist** is compiled **first**, as its own section of
  `34-AUDIT.md`, before any page is judged. Its mechanical sources, in precedence order:
  (1) `git diff v0.9.0..HEAD -- .project/current-exports.txt` — every added, removed or renamed
  public item; (2) `CHANGELOG.md` `[0.10.0]`; (3) `MIGRATION.md` §9.1 (behavioral changes), §9.2
  (public types), §9.3 (deps), §9.4 (migrations), §9.5 (config keys and env vars), §9.6 (routes
  and stream payloads), §9.7 (deprecations), §9.8 (operator checklist); (4) the
  `.planning/REQUIREMENTS.md` v0.10.0 capability list (prefixes `ENG`, `HITL`, `PLAT`, `OBS`,
  `SHIP`, `VOCAB`, `ACCT`, `PRIM`, `COMM`, `BUG`) as the *capability* axis. Per-phase `NN-CONTEXT.md`
  files are read only to clarify a specific item's meaning, and `NN-SUMMARY.md` files are read
  frontmatter-only (universal anti-pattern rules 6-7). The checklist is grouped by phase so a
  verdict can cite "Phase 27, `POST /v1/runs`" directly.
- **D-09:** The **Upgrading page and migration pointers** (`docs/src/api-reference/upgrading.md`,
  `migration-guide.md`) are checked row-for-row against `MIGRATION.md` §9.1 and §9.8 — Phase 35
  SC1 requires them to agree, so the audit records each disagreement as its own `MB-nn`.
- **D-10:** The **vocabulary sweep** is part of the mdBook partition: `grep -rniE
  '\bQuartermaster\b' docs/src` must be empty (Phase 30 D-14), and every page in the Phase 31 D-29
  `token_count` hit list is re-checked for a bare total beside a `TokenUsage` split. The "three
  ubiquitous-language lists" Phase 35 SC4 names are taken to be the naming table in
  `.github/copilot-instructions.md`, the term table in `.planning/PROJECT.md`, and
  `docs/src/architecture/domain-model.md`; the researcher confirms this identification and records
  it in `34-AUDIT.md`'s header.
- **D-11:** The **mdBook build itself is measured as a baseline**: the exact `docs.yml` sequence
  (`mdbook-mermaid install docs/` → `mdbook build docs/` with `[output.linkcheck]`
  `warning-policy = "error"` → `scripts/check-doc-examples.sh` → `scripts/check-doc-config.sh`),
  with local tool versions recorded against the pins (`mdbook 0.4.40`, `mdbook-mermaid 0.13.0`,
  `mdbook-linkcheck 0.7.7`). Any red step is a finding routed to Phase 35 (build/link) or Phase 36
  (`doc-examples` compile).

### Rustdoc measurement protocol
- **D-12:** Two commands are quoted **verbatim** in `34-AUDIT.md` as the bar Phase 36 must clear:
  the `ci.yml:63` line
  `cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt`
  and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`. Both are run
  locally on the recorded HEAD with `cargo --version` / `rustc --version` recorded (the
  devcontainer carries cargo 1.97.1; CI's lint job toolchain is recorded from `ci.yml` alongside,
  and any count difference between the two toolchains is itself a finding).
- **D-13:** Every `warning:` line from the first command is **enumerated as a row** — crate, file,
  line, warning kind (private intra-doc link, unclosed HTML tag, missing docs, unresolved link,
  other) — parsed from the captured output's `-->` locations, never summarised as a count.
  The count (73 at Phase 33 close, 72 at Phase 29, 16 under `--all-features` at Phase 31) is
  reported **alongside** the rows with its HEAD SHA, so drift is visible.
- **D-14:** The `-D warnings --all-features` run **aborts at the first failing crate in build
  order** (`paladin-ai-core`, 14 unresolved links at Phase 32-05), so its output is a floor, not
  the enumeration. The audit therefore **also runs per crate**:
  `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` for each of the eleven
  library crates plus the facade (`paladin-ai`), and enumerates every error from every crate. The
  known `[HeuristicTokenCounter]` link at `crates/paladin-memory/src/token_counter/mod.rs:3`
  (WINDOWS.md row 37) must appear in that enumeration or the method is wrong.
- **D-15:** **Doctests are measured as a baseline too** — `cargo test --workspace --doc` under the
  default feature set — because the coverage and `--tests` gates skip doctests (project memory)
  and Phase 36 SC3 requires them green. Any red doctest is an `RD-nn` row. Public items added in
  Phases 22-33 that lack a doc test are listed under the Phase 16 D-05 entry-point rule only
  (`pub *Builder` / `*Port` / `*Service`), via `scripts/check-public-api-examples.sh`; the audit
  does not invent a broader doc-test mandate.

### Examples build and currency protocol
- **D-16:** Build status is measured with the **four `ci.yml:548-558` invocations verbatim** —
  `cargo build --examples --offline`; `--example vision_analysis --example vision_battalion
  --features "vision,llm-openai"`; `--example document_processing --features
  "content-processing"`; `--example http_service_host --features "web-server"` — because the bulk
  selector silently skips targets whose `required-features` are unmet (43 of 47 covered by the
  bare form). If `--offline` fails in the devcontainer for a registry-cache reason, the run is
  repeated without it and the deviation recorded in `34-EVIDENCE.md`. `crates/doc-examples` is
  built through `scripts/check-doc-examples.sh` (its Layer 1 is `cargo check` on the crate;
  Layer 1b is the README quick-example mirror check; Layer 2 is the inline-block scan).
  `crates/paladin-llm/examples/live_vendor_smoke.rs` is built with
  `cargo build -p paladin-llm --example live_vendor_smoke` under the features its
  `required-features` names.
- **D-17:** The **currency verdict for an example** is derived in three checks, each recorded in
  the row: (a) *obsolete API* — the program fails to build, or builds but names an item the
  shipped-surface checklist (D-08) marks removed/renamed (`TokenUsage::from_total`, bare
  `token_count`, `Quartermaster`, the legacy `garrison::TokenCounter` trait, `LimitSource`, the
  pre-Phase-33 `retrieve_context` return type, and every other §9.2 / exports-diff removal);
  (b) *capability mapping* — the program is mapped to the capability its file name and its
  `examples/README.md` section claim, and that claim is checked against the tree; (c) *gap list* —
  a separate table lists every Phase 22-33 capability (from the D-08 checklist's capability axis)
  with **no** program under `examples/` demonstrating it, each a candidate `EX-nn` sized `L`.
  `examples/README.md` is audited as a page in its own right (it already states "Rust 1.70" against
  an MSRV of 1.88) and is an `EX-nn` item.
- **D-18:** A `doc-examples` module row records which `docs/src` pages `{{#include}}` it (grep for
  the module's anchors) so Phase 35 knows which pages a module edit re-renders, and Phase 36 knows
  which module a page correction may need.

### Partition, sizing and deferred routing
- **D-19:** Partition rule: every `MB-nn` goes to Phase 35; every `RD-nn` and `EX-nn` goes to
  Phase 36; a finding that is neither (a behavioural bug noticed while reading, a wrong CI comment,
  a planning-corpus inaccuracy such as PROJECT.md's "no crate ships its own `examples/`") is
  written to **`.planning/phases/34-documentation-currency-audit/deferred-items.md`**, the
  phase-local register the Phase 25/28/31/32 executors established, with a one-line pointer from
  `34-AUDIT.md` §7. Nothing is absorbed into 35 or 36 by convenience. WINDOWS.md rows are added only
  through `gsd-tools` (anti-pattern rule 15), if at all.
- **D-20:** Requirement prefix: `DOCS-*` is spent (Phase 16). The planner mints **`CURR-*`**
  (documentation *currency*) for Phase 34 and, per the ROADMAP's "assigned under the Phase 34
  prefix" note for Phases 35-36, the same prefix carries those phases' requirements. Alternatives
  rejected: `AUDIT-*` (does not fit 35/36, which fix rather than audit), a `BOOK-*` / `RDOC-*` split
  (two prefixes for one work stream).
- **D-21:** The two Phase 34 work lists are **ordered for execution**: within each, `L` items that
  block others (a missing page that stale pages should link to; a rustdoc family whose fix
  pattern repeats) come first, then by page/crate order. The lists are the *input* to
  `/gsd-plan-phase 35` and `36`; those phases may re-batch but must close every ID.

### Read-only enforcement and tool hygiene
- **D-22:** Success Criterion 5 is proven mechanically, not asserted: before every commit the
  executor runs `git diff --stat $(git merge-base HEAD main)..HEAD -- . ':!.planning'` and records
  its empty output in `34-EVIDENCE.md`; the phase's final VERIFICATION repeats it over the whole
  phase range. Build products stay in `target/` and `docs/book/` (both ignored); `mdbook-mermaid
  install docs/` rewrites `docs/mermaid.min.js` / `docs/mermaid-init.js` — after running it the
  executor checks `git status --porcelain -- docs` is clean and, if it is not, restores the files
  with `git checkout -- docs/` and records that the pin drifted (a Phase 35 finding, not a
  Phase 34 edit). — **Reversibility:** one-way — a docs edit committed under this phase would
  falsify SC5 and the "audit is read-only" premise Phases 35-37 plan against; it cannot be undone
  without rewriting history.
- **D-23:** The audit records the **HEAD SHA it measured** in its header and every table repeats
  it; if the branch moves before Phase 35/36 plan, those phases re-run the D-12/D-16 commands and
  diff against the recorded rows rather than trusting the counts.

### Claude's Discretion
- The exact Markdown table columns beyond the mandatory ones (ID, classification, size, location,
  citing phase + shipped item, evidence anchor).
- How the shipped-surface checklist is laid out (one table per phase vs one table with a phase
  column), so long as a verdict can cite "Phase N, item".
- Whether `34-EVIDENCE.md` inlines the full 73-line `cargo doc` capture or links a
  `34-evidence/` subdirectory of raw logs (both live under `.planning/`).
- How many plans the phase needs (the three partitions plus the checklist and the work-list
  assembly suggest four to five, executable in two waves: checklist first, then the three
  partitions in parallel, then assembly).

### Folded Todos
- **Verify local `make coverage` reproduces CI's 82.39% figure** (todo 2026-08-13, score 0.6).
  Folded slice: `docs/src/contributing/testing-guide.md`'s Code Coverage section gets a verdict
  like every other page, and the audit records whether the page's commands still match
  `Makefile:251-256` and `ci.yml`'s coverage job. The end-to-end walk on a Docker machine stays
  the maintainer's item (no Docker in this devcontainer) and is not closed by this phase.
- **Evaluate replacing MinIO with RustFS** (todo 2026-09-13, score 0.6). Folded slice: every
  `docs/src` page and every example that names a MinIO image, tag, host or download URL
  (`appendix/minio-file-repository-setup.md`, `deployment/docker.md`, `deployment/kubernetes.md`,
  `getting-started/installation.md`, and any other grep hit) is checked for the
  quay.io pin quick task 260913-15w introduced; a page still naming the retired Docker Hub image
  or `dl.min.io` is `stale`. The RustFS evaluation itself is infrastructure and stays deferred.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition and the consumers of its output
- `.planning/ROADMAP.md` §"Phase 34: Documentation Currency Audit" — goal, five success criteria,
  sources; §"Phase 35" and §"Phase 36" — what the two work lists must be sufficient for.
- `.planning/REQUIREMENTS.md` — the v0.10.0 capability list (prefixes `ENG`, `HITL`, `PLAT`, `OBS`,
  `SHIP`, `VOCAB`, `ACCT`, `PRIM`, `COMM`, `BUG`) that is the capability axis of D-08; SHIP-04's
  "mdBook + rustdoc updated with no new broken intra-doc links" is the release-level statement
  this phase measures against; the prefix protocol (extension record, 2026-09-14).
- `.planning/STATE.md` — Phase 32/33 close notes on the 14 unresolved links and the 73 warnings.

### The bars being measured
- `.github/workflows/ci.yml:62-63` — the "Check documentation" step, quoted verbatim (D-12);
  `:541-558` — the four-invocation examples split and the comment explaining why (D-16).
- `.github/workflows/docs.yml` — the mdBook toolchain pins, the linkcheck backend, and the two
  scripts it runs (D-11).
- `docs/book.toml` — `[output.linkcheck] warning-policy = "error"`, `follow-web-links = false`.
- `scripts/check-doc-examples.sh` — the three-layer doc-examples gate (D-16).
- `scripts/check-doc-config.sh` — the YAML-block syntactic gate (D-11).
- `scripts/check-public-api-examples.sh` — the `# Examples` enforcer on the 76 entry points (D-15).
- `.planning/decisions/0033-cargo-doc-warning-bar.md` — the ratified zero-warning bar and its
  2026-08-24 amendment (D-00a).
- `Cargo.toml` `[[example]]` entries (lines ~444-463) — the four feature-gated example targets.

### The method being reused
- `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-DOCS-01-VERDICTS.md`
  — the eight signal classes, the "content not mtime" rule, the row format (D-07).
- `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-CONTEXT.md`
  — D-00e evidence bar, D-05/D-06 entry-point rule.
- `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-DOCS-03-ENTRY-POINTS.md`
  — the 76-item public-API entry-point enumeration (D-15).
- `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-LINKCHECK-REPORT.md`
  — the prior linkcheck baseline format (D-11).
- `.planning/codebase/CONVENTIONS.md` §"`# Examples` heading spelling" — the scoped house rule.

### The shipped surface (D-08 sources)
- `CHANGELOG.md` `[0.10.0]` section.
- `MIGRATION.md` §9.1-§9.8.
- `.project/current-exports.txt` and `git diff v0.9.0..HEAD -- .project/current-exports.txt`.
- `.planning/phases/30-token-economy-vocabulary-commissary-anchoring/30-CONTEXT.md` — D-01 vocabulary
  rule, D-14 `Quartermaster` exit grep, D-10 `max_tokens` table location.
- `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` — D-08 bare-count rule, D-29 the
  `token_count` doc hit list.
- `.planning/phases/32-unified-token-primitives/32-CONTEXT.md` — D-01/D-02 `resolve_context_window`
  / `WindowSource` (currently named on zero mdBook pages), D-09/D-10 deleted `TokenCounter` trait.
- `.planning/phases/33-commissary-in-tree-adoption/33-CONTEXT.md` — D-03 crate-graph docs follow
  the new edge, D-12 `RagRetrievalResult`.
- `.planning/PROJECT.md`, `.github/copilot-instructions.md` naming table,
  `docs/src/architecture/domain-model.md` — the three ubiquitous-language lists (D-10, to confirm).

### Prior measurements of the rustdoc and examples state
- `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` row 26 and §"The `cargo doc
  --workspace --no-deps` condition — carried, not a gate" — the 73-warning baseline.
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` §8 and §11 — the 72-warning
  baseline and the re-seal.
- `.planning/phases/32-unified-token-primitives/32-05-SUMMARY.md` — the 14 unresolved links in
  `paladin-ai-core` (graph-fingerprinting and webhook-delivery doc families) and the build-order
  abort (D-14).
- `.planning/phases/32-unified-token-primitives/32-03-SUMMARY.md` and
  `.planning/phases/31-lossless-token-accounting/deferred-items.md` §Plan 31-05 — the
  `[HeuristicTokenCounter]` link and the 16-warning `--all-features` enumeration with file list.
- `.planning/WINDOWS.md` rows 36 and 37 — the open rustdoc rows Phase 36 closes.
- `.planning/phases/28-observability-tooling/28-17-PLAN.md` and
  `.planning/phases/29-program-gates-release/29-06-PLAN.md` — the two docs plans whose declared
  scope bounds define "what v0.9.0 → v0.10.0 docs work already happened" (29-06 explicitly left
  the default-feature `cargo doc` warning set out of scope).

### The surfaces themselves
- `docs/src/SUMMARY.md` — the nav; every page not listed here is an orphan (D-05).
- `examples/README.md` — the examples index, audited as a page (D-17).
- `crates/doc-examples/Cargo.toml` and `crates/doc-examples/src/*.rs` — the 11 compile-verified
  modules (D-18).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Phase 16's verdict record** (`16-DOCS-01-VERDICTS.md`) is a proven per-page audit format with a
  producing-command-per-signal discipline; D-07 adopts it and adds one signal class.
- **`NN-CI-EVIDENCE.md` pattern** (Phases 28, 29, 33): a numbered table of exact command → result
  → verdict, with verbatim captures below. `34-EVIDENCE.md` follows it (D-02).
- **`scripts/check-doc-examples.sh`, `check-doc-config.sh`, `check-public-api-examples.sh`** are the
  existing mechanical gates; the audit runs them rather than re-implementing their checks.
- **`.project/current-exports.txt`** with the `v0.9.0` tag gives a mechanical public-surface diff
  for the whole milestone (D-08).
- **Phase-local `deferred-items.md`** (Phases 25, 28, 31, 32) is the established out-of-scope
  register (D-19).

### Established Patterns
- **Five mdBook pages carry `Since: v0.10.0` markers** (`platform-api.md`, `wargraph-doc-schema.md`,
  `observability.md`, `eval-harness.md`, `graph-visualization.md`); dedicated pages exist for
  Phases 23-26 (`control-flow.md`, `parley-and-chronicle.md`, `fault-tolerance.md`,
  `agent-runtime.md`), Phase 27 (`platform-api.md`), Phase 28 (three pages), Phase 30
  (`architecture/commissary.md`). **No page in `SUMMARY.md` is dedicated to the Phase 22 superstep
  engine** (`WarEngine` / `Battlefield` / `Waypoint` appear on 8-10 pages only in passing) — a
  likely `missing` candidate the audit must confirm by content, not by this grep.
- **Phase 31-33 touched 19 `docs/src` pages** (2026-09-13 onward); **25 pages were last touched on
  or before 2026-06-03**, mostly `appendix/`. Neither date settles a verdict (D-00b) but both
  tell the planner where reading effort concentrates.
- **`docs/src/appendix/doc-coverage-report.md`** is a frozen 2026-05-28 report asserting "docs build
  succeeds with no warnings" — contradicted by the 73-warning baseline; a `stale` row.
- **Examples:** 27 of 48 programs were last touched 2026-09-15 (the Phase 31 usage rename), 21 are
  untouched since 2026-08-12 or earlier; only `war_engine_memory_baseline.rs` and
  `muster_baseline.rs` name a Phase 22+ concept, so the D-17(c) gap list is expected to be long.
  `examples/README.md` states "Rust 1.70" (MSRV is 1.88).
- **Local toolchain:** cargo/rustc 1.97.1; `mdbook`, `mdbook-linkcheck`, `mdbook-mermaid` are
  installed at `/usr/local/cargo/bin` (versions to be recorded against the `docs.yml` pins).

### Integration Points
- The two work lists are consumed by `/gsd-plan-phase 35` and `/gsd-plan-phase 36`; the ROADMAP
  marks those phases "Requirements: TBD — assigned at planning under the Phase 34 prefix" (D-20).
- `WINDOWS.md` rows 36/37 and the Phase 33 CI-evidence row 26 are the prior records the audit
  supersedes with an enumeration; Phase 36 SC1 closes row 36.
- Phase 37 re-seals the Phase 29 gates on the post-documentation commit; the audit's HEAD SHA
  (D-23) is the "before" reference for that re-seal's docs delta.

</code_context>

<specifics>
## Specific Ideas

- The audit header should read like `33-CI-EVIDENCE.md`'s: HEAD SHA, date, `cargo --version`,
  `rustc --version`, `mdbook --version`, `mdbook-linkcheck --version`, `mdbook-mermaid --version`,
  and the verbatim command list, so a reader can reproduce every row.
- Verdict rows cite in the form `Phase 27 — POST /v1/runs (PLAT-02)` so the phase, the shipped
  item and the requirement are all one grep away.
- The known-answer checks (D-14's `[HeuristicTokenCounter]`, the 14 `paladin-ai-core` links, the
  `doc-coverage-report.md` contradiction, the README "Rust 1.70") double as method self-tests: if
  the tables do not contain them, the sweep missed something.

</specifics>

<deferred>
## Deferred Ideas

- **PROJECT.md corpus correction** — PROJECT.md states "no crate under `crates/` ships its own
  `examples/` directory"; `crates/paladin-llm/examples/live_vendor_smoke.rs` exists. Record in
  `deferred-items.md`; it is a planning-corpus fact, not a docs item.
- **`docs/src/appendix/doc-coverage-report.md` disposition** — archive with a signpost (the Phase 16
  D-01 pattern) or regenerate from a real measurement. The audit records it `stale`; Phase 35
  decides.
- **A rustdoc gate in `make clean-code` / the pre-push hook** — Phase 36 SC2 owns this; the audit
  only notes that `make doc` today opens the browser and does not fail on warnings.
- **Toolchain drift between the devcontainer (1.97.1) and CI's lint job** — if D-12's counts differ
  between the two, the difference is recorded; choosing a pinned lint toolchain is a Phase 36 or
  later decision.

### Reviewed Todos (not folded)
- None — both matched todos were folded to their documentation slice (see Folded Todos); their
  non-documentation remainders (the Docker-machine coverage walk; the RustFS evaluation) stay
  open as pending todos, unchanged.

</deferred>

---

*Phase: 34-documentation-currency-audit*
*Context gathered: 2026-09-17*
