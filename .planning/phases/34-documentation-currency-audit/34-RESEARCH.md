# Phase 34: Documentation Currency Audit - Research

**Researched:** 2026-09-17
**Domain:** Read-only measurement of mdBook / rustdoc / examples currency against a Rust workspace
**Confidence:** HIGH (every figure below was reproduced live at HEAD `c36b7729` in this
devcontainer; nothing here is training-data recall)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
D-00a…D-00g (inherited, not re-litigated): `cargo doc` bar is zero `warning:` lines on
`cargo doc --workspace --no-deps` (ratified ADR-0033); a currency verdict is settled by content,
never file existence/mtime; audit findings are recorded, never fixed silently (fix set empty by
construction); WINDOWS.md rows are never deleted, only moved to fixed/waived; the `# Examples`
(plural) heading rule applies only to the D-05-enumerated public-API entry points; vocabulary rule
is Phase 30 D-01, `Quartermaster` purged, bare-count rule is Phase 31 D-08; shipped tree outranks
any document on disagreement.

D-01…D-23 (this phase's own decisions — see `34-CONTEXT.md` for full text): one canonical file
`34-AUDIT.md` (header, mdBook table, rustdoc table, examples table, Phase 35 work list, Phase 36
work list, deferred-routing pointers); verbatim tool output in `34-EVIDENCE.md`; stable IDs
`MB-nn`/`RD-nn`/`EX-nn` never renumbered; sizing S/M/L by edit kind; mdBook scope is all 93
`docs/src/**/*.md` files including orphans; verdict is `current`/`stale`/`missing`, every
`stale`/`missing` cites phase + shipped item; evidence is the Phase 16 eight signal classes plus a
ninth (shipped-surface checklist); shipped-surface checklist compiled first, sourced in order from
the `.project/current-exports.txt` diff, `CHANGELOG.md [0.10.0]`, `MIGRATION.md` §9.1-§9.8, then
`REQUIREMENTS.md`'s v0.10.0 capability list; Upgrading/migration pages checked row-for-row against
`MIGRATION.md` §9.1/§9.8; vocabulary sweep (`Quartermaster` grep, `token_count` hit list) is part
of the mdBook partition; mdBook build itself measured as a baseline (`mdbook-mermaid install` →
`mdbook build` with linkcheck → `check-doc-examples.sh` → `check-doc-config.sh`); two rustdoc
commands quoted verbatim as the Phase 36 bar; every `warning:`/error line enumerated as a row, not
summarised; the `-D warnings --all-features` run also run per-crate for all 11 library crates plus
the facade; doctests measured as a baseline (`cargo test --workspace --doc`); examples build status
measured with the four `ci.yml:548-558` invocations verbatim plus the `doc-examples` and
`live_vendor_smoke` builds; example currency verdict is (a) obsolete-API, (b) capability-mapping,
(c) gap-list; partition rule sends every `MB-nn` to Phase 35, every `RD-nn`/`EX-nn` to Phase 36,
anything else to `deferred-items.md`; requirement prefix is `CURR-*` (`DOCS-*` spent); work lists
ordered L-items-that-block-others first; Success Criterion 5 proven mechanically via
`git diff --stat` against `. ':!.planning'`, empty, recorded per-commit and at phase close; HEAD SHA
recorded in the audit header and repeated in every table.

### Claude's Discretion
Table columns beyond the mandatory ones; shipped-surface checklist layout (one table per phase vs.
one table with a phase column); whether `34-EVIDENCE.md` inlines the full captures or links a
`34-evidence/` subdirectory; how many plans (three partitions plus the checklist and the work-list
assembly suggest four to five, in two waves: checklist first, then the three partitions in
parallel, then assembly).

### Deferred Ideas (OUT OF SCOPE)
PROJECT.md's "no crate ships its own `examples/`" corpus error (record in `deferred-items.md`);
`doc-coverage-report.md` disposition (archive vs regenerate — Phase 35 decides, this phase records
`stale`); a rustdoc gate in `make clean-code`/pre-push (Phase 36 owns); toolchain-drift
decision-making (record only — **this research found there is no drift, see Finding F-12**).
</user_constraints>

<phase_requirements>
## Phase Requirements

No requirement IDs exist yet — `DOCS-*` is spent and CONTEXT.md D-20 mints `CURR-*`. Proposed
mapping of the five ROADMAP success criteria onto five `CURR-*` requirements (1:1, matching the
"3-5" range the task brief asked for):

| ID | Description | Research Support |
|----|-------------|------------------|
| CURR-01 | Every `.md` under `docs/src/` (93 files) gets a `current`/`stale`/`missing` verdict against the Phase 22-33 shipped surface, cited by phase + shipped item | F-01 (93-file count reproduced), F-02 (zero real orphans today — recount before writing rows), F-03 (Since:-marker page paths corrected), F-13 (Quartermaster grep is not clean), F-14 (`token_count` D-29 hit list), D-08 mechanical checklist sources verified live (F-15…F-18) |
| CURR-02 | Every `warning:` line from `cargo doc --workspace --no-deps` and every error from the per-crate `-D warnings --all-features` sweep enumerated with crate/file/line | F-04…F-11 (exact counts, kind breakdown, per-crate breakdown, the no-`-->`-location pitfall, the three-crate-not-one-crate abort correction) |
| CURR-03 | Every `examples/` program (48) and `doc-examples` module (11) recorded with build status per CI's four feature-set invocations plus a currency verdict | F-19…F-24 (all four + two extra builds green, `--offline` works, the 47-vs-48 CI-comment drift, doc-examples module list, README's stale "Rust 1.70") |
| CURR-04 | Findings partitioned into sized Phase 35 (`MB-nn`) / Phase 36 (`RD-nn`, `EX-nn`) work lists, non-doc findings routed to `deferred-items.md` | Reuses the `16-DOCS-01-VERDICTS.md` row schema (F-25) and `NN-CI-EVIDENCE.md` table schema (F-26); F-27 (check-public-api-examples.sh scope has drifted 76→101 items, itself a deferred-register candidate, not an MB/RD/EX item) |
| CURR-05 | The audit is read-only outside `.planning/`; `git diff --stat … ':!.planning'` is empty at every commit and at phase close | F-28 (`mdbook-mermaid install docs/` produced zero drift this run — `git status --porcelain -- docs` was clean before and after); verified working tree clean at research close |

</phase_requirements>

## Summary

This phase is a measurement exercise, not a library-integration one — there is no new dependency
to research, no framework to learn. What the planner needs is exact, reproduced command output
shapes and counts so the four work-producing plans (checklist, mdBook sweep, rustdoc sweep,
examples sweep) can be sized without re-discovering toolchain quirks live. Every command named in
CONTEXT.md D-08/D-11/D-12/D-14/D-16 was run against HEAD `c36b7729` in this devcontainer during this
research session; the counts below are live, not carried from STATE.md.

The single most important correction to CONTEXT.md's own assumptions: the `-D warnings
--all-features --workspace` command does **not** abort at one crate (`paladin-ai-core`) as D-14's
prose implies — it aborts after **three** crates fail (`paladin-ports`, `paladin-ai-core`,
`paladin-storage`, 16 content errors total) because cargo documents crates in parallel and only
stops accepting new work once any job fails. The per-crate sweep D-14 also mandates is therefore
not a nice-to-have but the **only** way to get the true floor, and it revealed the real one: **8 of
12 crates are RED under `-D warnings --all-features`, totaling 77 content errors** — nearly 5x the
16-warning figure Phase 31 recorded and the number this phase's own CONTEXT.md and ROADMAP still
cite ("14 unresolved intra-doc links"). Phase 36 is sizing against a stale, much-too-small number
unless this audit corrects it.

The second most important finding: roughly half of all `unresolved link` rustdoc warnings (34 of
36 in the default-feature run) carry **no `-->` file:line at all** — rustdoc's own diagnostic
output only gives a text snippet ("the link appears in this line: ...") when the broken link lives
inside a `//!` module/crate-level doc comment, as opposed to an `///` item-level one, which does get
a `-->`. D-13 asks for crate/file/line on every row; the mechanical way to get it for these rows is
to `grep -rn` the snippet text against the crate's source, not to parse the rustdoc output directly.
This method was validated against the one known-answer case (`HeuristicTokenCounter`,
`crates/paladin-memory/src/token_counter/mod.rs:3`, WINDOWS.md row 37) and reproduces the correct
line.

Third: the toolchain-drift question D-12 asks about has a clean answer — there is no drift.
`rust-toolchain.toml`'s own header comment states it "overrides whatever toolchain a workflow
action installed," and the `lint` job's `dtolnay/rust-toolchain@stable` step is exactly such an
override target; both environments actually run `cargo`/`rustc` 1.97.1. Record this as a closed
question, not an open one.

**Primary recommendation:** Plan the checklist-compilation task to lean on `CHANGELOG.md [0.10.0]`
(427 lines, 6 headed subsections) as the primary readable D-08 source, using the
`.project/current-exports.txt` diff (4,376 raw added lines against the v0.9.0 tag) only as a
cross-check grep target, never as a line-by-line reading source — it is too large to read
end-to-end and is not phase-attributed.

## Architectural Responsibility Map

This phase touches no runtime architecture — it is a documentation/measurement audit over an
already-shipped tree. The "capabilities" here are audit responsibilities, not application tiers:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| mdBook currency verdicts | Documentation corpus (`docs/src/`) | Planning corpus (`.planning/`) for the audit record itself | The mdBook is a standalone static-site tier; the audit's own output lives entirely under `.planning/` per D-22/SC5 |
| Rustdoc warning/error enumeration | Rust source tier (doc comments in `crates/*/src`, `src/`) | CI (`lint` job, `docs.yml`) as the bar-of-record | Findings point at source `///`/`//!` comments; the CI job is what Phase 36 must turn green, not something this phase touches |
| Examples build/currency | `examples/`, `crates/doc-examples/`, `crates/paladin-llm/examples/` | `Cargo.toml` `[[example]]` targets as the feature-gate declaration | Build status is a property of the example source + its Cargo-declared `required-features`, not of any runtime tier |
| Shipped-surface checklist | Planning corpus (`CHANGELOG.md`, `MIGRATION.md`, `REQUIREMENTS.md`) | `.project/current-exports.txt` (mechanical cross-check) | The checklist is a read-only synthesis of already-written release documents, not a re-derivation from source |

## Standard Stack

No new dependency is introduced by this phase (D-00c/SC5: read-only, `.planning/`-only commits).
The "stack" is the set of already-vendored, already-pinned tools this phase invokes to take
measurements:

### Core (already in the tree — versions verified live, not assumed)

| Tool | Version (measured, this devcontainer) | Pin source | Purpose |
|------|------|------------|---------|
| `cargo`/`rustc` | 1.97.1 | `rust-toolchain.toml` `channel = "1.97.1"` | `cargo doc`, `cargo test --doc`, `cargo build --examples` |
| `mdbook` | 0.4.40 | `docs.yml:46` `--version 0.4.40 --locked` | `mdbook build docs/` |
| `mdbook-linkcheck` | 0.7.7 | `docs.yml:54` `--version 0.7.7 --locked` | linkcheck backend, `[output.linkcheck] warning-policy = "error"` |
| `mdbook-mermaid` | 0.13.0 | `docs.yml:50` `--version 0.13.0 --locked` | diagram rendering, `mdbook-mermaid install docs/` |
| `cargo-public-api` | 0.52.0 (as recorded in `.project/current-exports.txt` header) | n/a (generator of the exports file, not re-run this phase) | source of the D-08 exports diff |

**Version verification:** All three mdBook-family tool versions were confirmed by running
`mdbook --version` / `mdbook-linkcheck --version` / `mdbook-mermaid --version` directly in this
devcontainer — they match `docs.yml`'s pins exactly (`[VERIFIED: local binary + docs.yml]`). The
`cargo --version`/`rustc --version` pair (1.97.1) matches `rust-toolchain.toml` exactly
(`[VERIFIED: local binary + rust-toolchain.toml]`).

### Supporting (mechanical gate scripts, already in the tree)

| Script | Purpose | When to Use |
|--------|---------|-------------|
| `scripts/check-doc-examples.sh` | Layer 1: `cargo check` on `crates/doc-examples`; Layer 1b: README quickstart mirror check; Layer 2: inline fenced-block syntax scan | D-11 mdBook-build baseline, D-18 doc-examples module mapping |
| `scripts/check-doc-config.sh` | YAML fenced-block syntactic gate | D-11 mdBook-build baseline |
| `scripts/check-public-api-examples.sh` | `# Examples`-heading enforcer on the D-05 entry-point set (dynamically re-derived, **not** frozen at the 76-item `16-DOCS-03-ENTRY-POINTS.md` list) | D-00e's scoping note — see Pitfall P-06, this is currently RED and **not wired into CI** |

### Alternatives Considered

Not applicable — this phase adds no library. The only "alternative" is method choice (grep-derived
file:line vs. trusting rustdoc's own `-->` output for every warning), covered in Pitfalls below.

**Installation:** None required — every tool above is already installed and pinned in this
devcontainer; no `cargo install` or `pip install` step is needed to execute this phase's plans.

## Package Legitimacy Audit

Not applicable — this phase installs no new package (D-00c/SC5 forbid any change outside
`.planning/`, and no new dependency is needed to run `cargo doc`, `mdbook build`, or the existing
shell scripts).

## Architecture Patterns

### System Architecture Diagram

```
   HEAD c36b7729 (frozen tree)
         |
         v
  +-----------------------------+
  | D-08 shipped-surface        |   sources, in precedence order:
  | checklist compilation       |   1. current-exports.txt diff (v0.9.0..HEAD)  [cross-check only]
  | (own section of 34-AUDIT.md)|   2. CHANGELOG.md [0.10.0]                    [primary reading]
  +-----------------------------+   3. MIGRATION.md §9.1-9.8                    [primary reading]
         |                          4. REQUIREMENTS.md v0.10.0 capability list  [capability axis]
         v
  +----------------+   +----------------+   +----------------+
  | mdBook partition|   | rustdoc partition|  | examples partition|
  | (93 docs/src    |   | (cargo doc x2    |  | (48 examples/ +   |
  |  pages, vs      |   |  workspace +     |  |  11 doc-examples  |
  |  checklist)     |   |  12 per-crate    |  |  modules + 1      |
  |                 |   |  sweeps, vs      |  |  live_vendor_smoke|
  |                 |   |  checklist)      |  |  vs checklist)    |
  +--------+--------+   +--------+---------+  +---------+---------+
           |                      |                      |
           v                      v                      v
       MB-nn rows             RD-nn rows              EX-nn rows
           |                      |                      |
           +----------+-----------+----------+-----------+
                      |                       |
                      v                       v
             Phase 35 work list       Phase 36 work list
                      |                       |
                      +-----------+-----------+
                                  v
                     deferred-items.md (non-doc findings)
```

### Recommended Plan/Artifact Structure

```
.planning/phases/34-documentation-currency-audit/
├── 34-CONTEXT.md                 # already exists — locked decisions
├── 34-RESEARCH.md                # this file
├── 34-AUDIT.md                   # the single canonical inventory (D-01)
├── 34-EVIDENCE.md                # verbatim tool captures (D-02) — or a 34-evidence/ subdir
├── deferred-items.md             # non-doc findings (D-19)
├── 34-0N-PLAN.md / SUMMARY.md    # 4-5 plans per Claude's Discretion
```

### Pattern 1: Shipped-surface checklist compiled before any verdict (D-08)

**What:** A standalone section of `34-AUDIT.md`, grouped by phase, built from `CHANGELOG.md
[0.10.0]`'s 6 subsections (`Behavioral changes`, `Changed`, `Added`, `Removed`, `Fixed`, `Known
limitations` — 427 lines total) and `MIGRATION.md`'s 8 numbered subsections (§9.1 148 lines, §9.2
125 lines, §9.3 24 lines, §9.4 25 lines, §9.5 82 lines, §9.6 253 lines — the largest, the HTTP
surface — §9.7 6 lines, §9.8 to EOF; 722 lines total), cross-checked (not read line-by-line) against
`.project/current-exports.txt`'s diff.

**When to use:** First plan of the phase, before any of the three partition plans start — the
mdBook/rustdoc/examples verdicts all cite into this checklist.

**Example (verified command, not code):**
```bash
# Source 1 (cross-check only — 7,924 lines new / 3,550 lines at v0.9.0, ~4,376 net additions,
# too large to read end-to-end; use as a grep target for specific type/fn names, not prose)
git diff v0.9.0..HEAD --stat -- .project/current-exports.txt

# Source 2 (primary reading — 427 lines, 6 subsections)
awk '/^## \[0\.10\.0\]/,/^## \[0\.9\.0\]/' CHANGELOG.md

# Source 3 (primary reading — 722 lines total, 8 subsections)
grep -n '^## 9\.' MIGRATION.md
```

### Pattern 2: Per-crate `-D warnings --all-features` sweep, not just the workspace command (D-14)

**What:** `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` run once per
crate name (the 11 library crates plus the facade `paladin-ai` — 12 invocations total), because the
single `--workspace` invocation only surfaces the first ~3 crates that fail before cargo stops
scheduling new documentation jobs.

**When to use:** The rustdoc partition plan, immediately after the two workspace-level commands
D-12 quotes verbatim.

**Example (verified command and live result table):**
```bash
for c in paladin-battalion paladin-content paladin-ai-core paladin-eval paladin-herald \
         paladin-llm paladin-memory paladin-notifications paladin-ports paladin-storage \
         paladin-web paladin-ai; do
  RUSTDOCFLAGS="-D warnings" cargo doc -p "$c" --all-features --no-deps
done
```

| Crate | rc | Content errors | Wall time |
|-------|----|----------------|-----------|
| paladin-battalion | 101 | 36 (21 unresolved-link, 12 private-link, 3 redundant-link) | 30s |
| paladin-content | 0 | 0 | 36s |
| paladin-ai-core | 101 | 14 (all unresolved-link) | 7s |
| paladin-eval | 0 | 0 | 50s |
| paladin-herald | 0 | 0 | 5s |
| paladin-llm | 101 | 9 (7 private-link, 2 unclosed-HTML) | 8s |
| paladin-memory | 101 | 1 (unresolved-link — the known `HeuristicTokenCounter`) | 7s |
| paladin-notifications | 0 | 0 | 28s |
| paladin-ports | 101 | 1 (private-link) | 5s |
| paladin-storage | 101 | 1 (private-link) | 72s |
| paladin-web | 101 | 8 (3 unresolved-link, 5 private-link) | 8s |
| paladin-ai (facade) | 101 | 7 (all private-link) | 51s |
| **Total** | — | **77** | **4m 47s** |

This is the true floor D-14 asks for. It is **not** "14 unresolved links in `paladin-ai-core`" as
CONTEXT.md/ROADMAP/STATE.md currently state — that 14-count is still correct for `paladin-ai-core`
alone, but 63 more content errors exist in the other 7 red crates that the single `--workspace`
command's 16-error partial output (3 crates, 16 errors) never reached.

### Pattern 3: Locating file:line for a location-less rustdoc warning (see Pitfall P-01)

**What:** `grep -rn "<exact text from the 'the link appears in this line:' snippet>"
crates/<crate>/src src` to recover the file and line rustdoc's own diagnostic omits for links
broken inside `//!` (module/crate-level) doc comments.

**Example (verified against the WINDOWS.md row 37 known-answer case):**
```bash
$ grep -n "HeuristicTokenCounter" crates/paladin-memory/src/token_counter/mod.rs
3://! [`HeuristicTokenCounter`] is the phase-wide default: a synchronous,
```
Reproduces `crates/paladin-memory/src/token_counter/mod.rs:3` exactly, matching WINDOWS.md row 37.

### Anti-Patterns to Avoid
- **Trusting `-->` presence as universal:** only 29 of 65 default-feature-run warnings and only a
  minority of the 77 all-features-run errors carry a `-->` line at all (see Pitfall P-01). A row
  schema that assumes every warning has a machine-parseable location will silently drop rows.
- **Reading `.project/current-exports.txt`'s diff prose-style:** at 4,376 added lines it is a
  cross-check grep target, not a reading source; `CHANGELOG.md [0.10.0]` (427 lines) is the
  readable summary of the same information, phase-attributed where the exports diff is not.
- **Assuming the CI job's stated "47 example files" is current:** `find examples -name '*.rs' |
  wc -l` returns 48 today; `.github/workflows/ci.yml:538`'s own comment is stale by one file (see
  Pitfall P-05). Verify the live count, don't copy the comment.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Detecting which doc-examples module backs which mdBook page | A fresh grep script | `grep -rn '{{#include ../../../crates/doc-examples' docs/src` against each module's `// ANCHOR:` names | The include-anchor convention is already load-bearing (`check-doc-examples.sh` Layer 1); reuse its anchor names rather than inventing a second mapping |
| Verifying a page's YAML config snippets | Custom Python | `scripts/check-doc-config.sh` (already runs, green, 0.9s, 154 blocks) | Existing gate already does exactly this; D-11 asks to run it, not reimplement it |
| Verifying doc-examples compile | A separate `cargo check` invocation per module | `scripts/check-doc-examples.sh` Layer 1 (already runs `cargo check` on the whole `paladin-doc-examples` crate, 38.6s) | One invocation covers all 11 modules; per-module invocation would be 11x slower for the same information |
| Enumerating the D-05 public-API entry-point set | A hand count from `16-DOCS-03-ENTRY-POINTS.md`'s 76 items | `scripts/check-public-api-examples.sh --list` (dynamically re-derives from the live tree — 101 items today, see Pitfall P-06) | The 76-item file is a frozen Phase 16 snapshot; the script is the mechanically current source, and the two have already drifted 76→101 |

**Key insight:** every mechanical check this phase needs already exists as a script in `scripts/`
or a `cargo`/`mdbook` subcommand. The work is running them, parsing their output into the D-03
row schema, and correcting the two-year-old assumptions (14-not-77 rustdoc errors, 47-not-48
examples, 76-not-101 entry points) that earlier phases' summaries carried forward without
re-measuring.

## Common Pitfalls

### Pitfall P-01: Most `unresolved link` rustdoc warnings carry no `--> file:line`
**What goes wrong:** A row-extraction script that `grep`s for `-->` immediately after `^warning:
unresolved link` will silently produce zero rows for the majority of this warning kind.
**Why it happens:** rustdoc emits a `-->` span only when it can resolve the doc comment to a
specific `///` item-attached span. Warnings originating inside a `//!` inner/module-level doc
comment are reported with only a `= note: the link appears in this line: <snippet>` block — no
file, no line. Measured live: 34 of 36 default-run `unresolved link` warnings, and the majority of
the all-features-run's unresolved-link errors, have no `-->` at all.
**How to avoid:** For any warning lacking a `-->`, `grep -rn "<the snippet text>"` against the
crate's `src/` to recover file:line (validated exactly against the WINDOWS.md row 37 known-answer
case — see Pattern 3).
**Warning signs:** A parsed row table with a suspiciously low row count for `unresolved link`
compared to the raw `grep -c "^warning: unresolved link"` count is the tell that this pitfall was
hit.

### Pitfall P-02: The all-features workspace command does not abort at exactly one crate
**What goes wrong:** D-14's own prose ("aborts at the first failing crate in build order
(`paladin-ai-core`, 14 unresolved links)") undersells the floor by 2 crates and 2 errors.
**Why it happens:** `cargo doc --workspace` schedules documentation jobs for independent crates
concurrently. When one job fails, cargo stops *scheduling new* jobs but lets already-started ones
finish, so however many crates were mid-flight at the failure moment also report their own errors
before the whole invocation exits 101. Measured live at HEAD `c36b7729`: `paladin-ports` (1 error),
`paladin-ai-core` (14 errors), `paladin-storage` (1 error) — 3 crates, 16 content errors — before
"error: could not document `paladin-storage`" and the final abort.
**How to avoid:** Always run the per-crate sweep (Pattern 2); never treat the single workspace
command's partial output as the floor's full extent, even for crate *count*.
**Warning signs:** If the workspace-level all-features run reports failures for more than one crate
name, the per-crate sweep is not optional — it is the only accurate enumeration.

### Pitfall P-03: The all-features rustdoc error count has grown ~5x since the last recorded figure
**What goes wrong:** Sizing Phase 36 against "14 unresolved links" (CONTEXT.md, ROADMAP.md,
STATE.md Phase 32 close note) undersizes the actual RD-nn work list by roughly 63 items.
**Why it happens:** Phase 31 measured 16 warnings under `--all-features`; by Phase 32 close the
number cited was "14 unresolved links in `paladin-ai-core`" (a narrower, single-crate framing that
was accurate for that one crate but was never re-run per-crate). No phase between 31 and 33 re-ran
the full per-crate `-D warnings --all-features` sweep.
**How to avoid:** Use this research's Pattern 2 table (77 total, 8 red crates) as the Phase 34
baseline; do not carry forward the 14/16 figures without the per-crate re-measurement this phase
performs.
**Warning signs:** Any RD-nn work-list total under ~70 items should be treated as under-enumerated
and re-checked against a fresh per-crate sweep at plan time (counts may have drifted further if
HEAD has moved — re-run, don't trust this document's numbers past D-23's re-run trigger).

### Pitfall P-04: `docs/src/appendix/doc-coverage-report.md` and `examples/README.md` self-contradict the live tree
**What goes wrong:** Trusting either file's own prose as evidence.
**Why it happens:** Both are frozen snapshots. `doc-coverage-report.md:18` states "Current result:
docs build succeeds with no warnings" — directly contradicted by the measured 73-warning
default-feature baseline. `examples/README.md:24` states "Rust 1.70 or later" against a measured
MSRV of 1.88 (`Cargo.toml` `[workspace.package] rust-version = "1.88"`).
**How to avoid:** Both are themselves `MB-nn`/`EX-nn` `stale` rows, not sources of truth — D-00b
already establishes this (content over mtime), this is just the concrete pair to record.
**Warning signs:** None needed — these are two of the phase's own known-answer self-tests
(CONTEXT.md "Specific Ideas"), reproduced here exactly.

### Pitfall P-05: `ci.yml`'s own comment about the examples count is stale
**What goes wrong:** Citing "examples/ holds 47 .rs files" (the literal `ci.yml:538` comment) as
the current count.
**Why it happens:** `find examples -name '*.rs' | wc -l` returns **48** today (confirmed via
`git log --diff-filter=A` — 48 files were ever added, none removed). A file was added after that
comment was last updated.
**How to avoid:** Always re-count live (`find examples -name '*.rs' | wc -l`); route the stale
comment itself to `deferred-items.md` per D-19's explicit example category ("a wrong CI comment"),
not into an `EX-nn` row (it is not an example file, it is a CI workflow comment).
**Warning signs:** A 47-vs-48 mismatch anywhere in a plan is this pitfall.

### Pitfall P-06: `check-public-api-examples.sh`'s enforced set has silently grown 76→101, is currently RED, and is not wired into CI
**What goes wrong:** Assuming the D-05/D-06 `# Examples` rule is either (a) still scoped to the 76
items `16-DOCS-03-ENTRY-POINTS.md` enumerated, or (b) currently passing, or (c) something CI would
catch if it regressed.
**Why it happens:** The script re-derives its target set live from the tree on every run (by
design — see its own header comment) rather than reading the frozen 76-item file. Phases 22-33 add
`pub *Builder`/`*Port`/`*Service` items faster than anyone re-runs the script. Measured live:
`--list` derives **101** entry points today, and default (gate) mode exits 1 with **19** `MISSING`
or `SINGULAR` violations (e.g. `RunTracePort`, `NodeCachePort`, `RunRepositoryPort`,
`WebhookDeliveryService`, `RunSubmissionService`, `AssistantService` and 13 more). `grep -rn
check-public-api-examples .github/workflows/*.yml Makefile` returns nothing — no CI job and no
`make` target runs it.
**How to avoid:** Per D-00e ("the audit reports on that set only; it does not extend the rule"),
this phase does **not** need to fix the 19 violations or expand `16-DOCS-03-ENTRY-POINTS.md` — but
it should record the drift and the 19-item RED result as a finding (it is neither an `MB-nn` mdBook
finding nor squarely an `RD-nn`/`EX-nn` rustdoc/example finding — recommend routing it to
`deferred-items.md` with a pointer, since it is a public-API-doc-coverage gap category the phase's
own D-03 ID taxonomy has no slot for).
**Warning signs:** Don't let a plan quietly "fix" any of the 19 MISSING/SINGULAR items — that would
violate D-00c (findings recorded, never fixed silently) since this rule's scope was never
formally re-litigated to 101 items.

### Pitfall P-07: `docs/src/architecture/commissary.md` contains a live `Quartermaster` hit
**What goes wrong:** Assuming D-10's `grep -rniE '\bQuartermaster\b' docs/src` returns empty (as
VOCAB-06 established for `crates`/`src`, a different scope).
**Why it happens:** `commissary.md:7` reads "...the Quartermaster→Commissary rename rationale...and
the rejected-name list are in ADR-0049" — a legitimate historical pointer sentence, not a stray
leftover, but it is a literal match for the grep D-10 specifies against `docs/src`.
**How to avoid:** Record this as a specific `MB-nn` finding with its own row rather than assuming
the grep is clean; let Phase 35 decide whether an ADR-pointer sentence is an intentional exception
to the "must be empty" rule or needs rewording (e.g., "the historic Quartermaster→Commissary
rename" is unambiguous without the bare, headword-matching form) — this research does not
prejudge that verdict, per this phase's own read-only/no-fix mandate.
**Warning signs:** A checklist item claiming "Quartermaster sweep: clean" without citing this one
hit is wrong; the correct claim is "1 hit, in an ADR-pointer sentence, verdict TBD by Phase 35."

### Pitfall P-08: `cli_isolation`'s `--all-features` test conflict is a `cargo test`, not a `cargo doc`, artifact
**What goes wrong:** Confusing this pre-existing, already-logged (`deferred-items.md`, Phase 31/32)
test failure with a rustdoc gate.
**Why it happens:** It only fires under `cargo test --workspace --all-features` (the
`test_cli_feature_is_not_default` assertion is contradicted once `--all-features` trivially enables
the `cli` feature) — it does **not** affect `cargo test --workspace --doc` under default features,
which this research ran clean (462 passed, 0 failed, 210 ignored).
**How to avoid:** Don't attempt `cargo test --workspace --all-features --doc` as a "more thorough"
doctest baseline — it isn't needed (D-15 specifies default features) and would spuriously fail on
this pre-existing, unrelated issue.
**Warning signs:** A doctest command line that adds `--all-features` where D-15 didn't ask for it.

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase. No stored data, live service
config, OS-registered state, secrets, or build artifacts are touched or renamed by this phase; it
performs zero writes outside `.planning/`.

## Code Examples

All "code examples" for this phase are shell commands, already given inline in Architecture
Patterns above and the Validation Architecture section below. There is no application source code
to write for this phase (D-00c/SC5).

## State of the Art

| Old Approach (what earlier phases recorded) | Current Approach (measured this session) | When Changed | Impact |
|--------------|------------------|--------------|--------|
| "14 unresolved intra-doc links" (`paladin-ai-core` only) as the `--all-features` floor | 77 content errors across 8 of 12 crates | Between Phase 32 close (2026-09-16) and this research (2026-09-17) — no intervening phase re-ran the per-crate sweep, so the number was simply never re-measured, not regressed by a specific commit | Phase 36's RD-nn work list must size against 77, not 14 |
| `cargo doc --workspace --no-deps` warning count "73 (Phase 33 close)" | 73, confirmed unchanged at HEAD `c36b7729` | No change | Confirms the default-feature bar is stable; only the all-features bar had drifted |
| Toolchain-drift treated as an open question (D-12: "any count difference...is itself a finding") | No drift — `rust-toolchain.toml` overrides the `lint` job's `dtolnay/rust-toolchain@stable`, both run 1.97.1 | Always true, just not previously stated explicitly | Closes the open question in D-12's own text; record as "no drift" rather than leaving it open |
| `examples/` "47 .rs files" (`ci.yml:538` comment) | 48 files on disk, all 48 build clean | A file was added after the comment was written | The CI comment itself is now a stale-doc finding (deferred, not `EX-nn`) |
| `16-DOCS-03-ENTRY-POINTS.md`'s "76-item enumeration" | `check-public-api-examples.sh --list` derives 101 items live, 19 RED | Continuous drift as Phases 22-33 added `pub *Builder`/`*Port`/`*Service` items | Out of this phase's fix scope (D-00e), but worth a deferred-register pointer |

**Deprecated/outdated:** The "14 unresolved links" and "47 examples" figures should not be copied
forward into `34-AUDIT.md` without the corrected counts this research provides.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The three "ubiquitous-language lists" D-10 names (`.github/copilot-instructions.md` naming table, `.planning/PROJECT.md` term table, `docs/src/architecture/domain-model.md`) are the correct identification — confirmed by grep to all three containing a `Commissary` row/entry, but not exhaustively proven to be the *only* three lists in the corpus | Architectural Responsibility Map / D-10 context | Low — CONTEXT.md already asks the researcher to "confirm or correct," and this research confirms via direct content match; a fourth undiscovered list would only add rows, not invalidate existing ones |
| A2 | The git-log-based "27 of 48 touched 2026-09-15" / "21 untouched since 2026-08-12" figures CONTEXT.md cites could not be exactly reproduced this session (a `%cd` committer-date tally returned 22 for the most recent date bucket, not 27) — likely an author-date vs. committer-date or `--follow`/rename-handling difference in the exact command used originally | Specific Ideas / examples inventory | Low — the discrepancy is in a descriptive count only, not a build-status or currency verdict; re-run whichever exact `git log` invocation the executor prefers at plan time and treat this document's 22 as provisional |

**If this table is empty:** N/A — two low-risk items recorded above; every counted/measured figure
in this document (warning counts, error counts, file counts, build results, tool versions) was
reproduced directly in this session and is `[VERIFIED: local command output]`, not `[ASSUMED]`.

## Open Questions

1. **Should the P-07 `Quartermaster` hit in `commissary.md` and the P-06 101-vs-76-item /
   19-violation `check-public-api-examples.sh` drift be `MB-nn` rows, or deferred-register entries?**
   - What we know: D-19's deferred category is "neither documentation nor an example" findings;
     both P-06 and P-07 findings *are* about documentation (a page's prose, and a doc-coverage
     script), so a literal reading argues for `MB-nn`.
   - What's unclear: P-06 is about the *rule's own scope drift* (76→101 items), not about any one
     page's content — closer to a meta-finding about the audit apparatus than a `docs/src` page
     currency verdict.
   - Recommendation: Route P-07 (`commissary.md`'s specific line) as an `MB-nn` row, since it names
     one page and one line. Route P-06 (the script's scope drift and its 19 current violations) to
     `deferred-items.md` with a pointer from `34-AUDIT.md` §7, since D-00e already says this phase
     "does not extend the rule" — recording a scope-drift finding without an ID category to close
     it against fits the deferred register's purpose better than inventing an ad hoc `RD`/`MB`
     classification for it.

2. **Does the all-features per-crate sweep need to be re-run at Phase 34 plan/execution time, given
   D-23's "HEAD moved, re-run" rule?**
   - What we know: This research measured at HEAD `c36b7729` (the same SHA `git status` showed at
     session start); no commits landed during this session.
   - What's unclear: Whether any commits land on `feature/phase-33` (or wherever Phase 34 executes)
     between this research being written and the phase's plans running.
   - Recommendation: Treat every count in this document as a **verified baseline for HEAD
     `c36b7729`**, and have the first Phase 34 plan re-run the D-12/D-14/D-16 commands fresh and
     diff against these tables per D-23, rather than copying these numbers into `34-AUDIT.md`
     unverified.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo`/`rustc` | All measurement commands | ✓ | 1.97.1 | — |
| `mdbook` | D-11 mdBook build baseline | ✓ | 0.4.40 (matches pin) | — |
| `mdbook-linkcheck` | D-11 linkcheck backend | ✓ | 0.7.7 (matches pin) | — |
| `mdbook-mermaid` | D-11 diagram install | ✓ | 0.13.0 (matches pin) | — |
| `python3` + `pyyaml` | `check-doc-config.sh` | ✓ | present, script ran green | — |
| Docker | Not required by this phase (no `docker`-gated test tier is part of D-08/D-11/D-12/D-14/D-16) | n/a | — | — |

**Missing dependencies with no fallback:** None — every tool this phase's plans need is already
present and correctly pinned in this devcontainer.

**Missing dependencies with fallback:** None.

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json` (only `workflow._auto_chain_active`
and `workflow.worktree_skip_hooks` are set) — treated as enabled per the standard default.

For a read-only audit phase, "tests" are mechanical completeness/consistency checks over
`34-AUDIT.md`/`34-EVIDENCE.md` themselves, plus the SC5 read-only proof — not application tests.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None (no application code) — plain shell/grep assertions over the audit's own Markdown output |
| Config file | none |
| Quick run command | `git diff --stat $(git merge-base HEAD main)..HEAD -- . ':!.planning'` — must be empty |
| Full suite command | The full command set below, run per plan and once at phase close |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CURR-01 | Every `docs/src/**/*.md` path appears as a row in `34-AUDIT.md`'s mdBook table | completeness grep | `comm -23 <(find docs/src -name '*.md' \| sed 's#docs/src/##' \| sort) <(grep -oE 'docs/src/\S+\.md' 34-AUDIT.md \| sed 's#docs/src/##' \| sort -u)` — must be empty | ❌ Wave 0 (34-AUDIT.md doesn't exist yet — this IS the phase's own deliverable) |
| CURR-02 | Every `warning:`/`error:` line in the captured `cargo doc` outputs has a matching table row | completeness grep | `grep -c '^warning:\|^error:' 34-evidence/*.txt` vs a row-count grep on `34-AUDIT.md`'s rustdoc table — counts must reconcile (summary "generated N warnings" lines excluded per Pattern 2) | ❌ Wave 0 |
| CURR-03 | Every `examples/*.rs` and `crates/doc-examples/src/*.rs` (excluding `lib.rs`) appears as a row | completeness grep | `comm -23 <(find examples -name '*.rs' -o -path '*/doc-examples/src/*.rs' ! -name lib.rs \| sort) <(grep -oE '\S+\.rs' 34-AUDIT.md \| sort -u)` — must be empty | ❌ Wave 0 |
| CURR-04 | Every `MB-nn`/`RD-nn`/`EX-nn` ID appears in exactly one of the two Phase-35/36 work lists, or in `deferred-items.md` | uniqueness + coverage grep | `grep -oE '(MB\|RD\|EX)-[0-9]+' 34-AUDIT.md \| sort \| uniq -c \| awk '$1>1'` — must be empty (no ID cited twice with different dispositions) | ❌ Wave 0 |
| CURR-05 | No file outside `.planning/` is modified by this phase's commits | read-only proof | `git diff --stat $(git merge-base HEAD main)..HEAD -- . ':!.planning'` — must be empty | ✓ (git itself) |

### Sampling Rate
- **Per task/plan commit:** the CURR-05 quick command (sub-second, run before every commit per
  D-22).
- **Per wave merge:** the relevant completeness grep for whichever partition(s) that wave produced.
- **Phase gate:** all five commands above, green, before the phase's own `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `34-AUDIT.md` does not exist yet — it is this phase's own primary deliverable, not a
  pre-existing test fixture; the checklist-compilation plan creates its skeleton (header + all 7
  D-01 sections) before the three partition plans can append rows.
- [ ] `34-EVIDENCE.md` (or a `34-evidence/` subdirectory) does not exist yet — the plan that runs
  each D-08/D-11/D-12/D-14/D-16 command for real must create it and anchor-reference it from
  `34-AUDIT.md`, per D-02.
- No test-framework install is needed — every completeness check above is a `grep`/`comm` one-liner
  over the phase's own Markdown output, runnable with tools already in this devcontainer.

## Security Domain

`security_enforcement` is not set in `.planning/config.json` — treated as enabled per default, but
this phase has no attack surface to evaluate: it makes zero writes outside `.planning/`, adds no
network calls, no new dependency, no new input-handling code, and no auth/session/crypto surface.
The ASVS categories are uniformly not-applicable:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a — no auth surface touched |
| V3 Session Management | no | n/a |
| V4 Access Control | no | n/a |
| V5 Input Validation | no | n/a — no new input path; the phase reads its own repo's already-trusted source tree |
| V6 Cryptography | no | n/a |

### Known Threat Patterns for this phase

None apply. The one place a security-adjacent judgment call exists is **not** hand-rolled scope
creep: `scripts/check-public-api-examples.sh`'s RED result (Pitfall P-06) touches no secret,
credential, or attacker-reachable surface — it is a documentation-coverage gate. No STRIDE row is
warranted.

## Sources

### Primary (HIGH confidence — reproduced live this session, HEAD `c36b7729`)
- `cargo doc --workspace --no-deps` — 73 warnings, 65 content findings, 8 summary lines, 42s
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` — aborts after 3
  crates (paladin-ports 1, paladin-ai-core 14, paladin-storage 1 = 16 errors), 23s
- 12x `RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --all-features --no-deps` (11 library crates
  + facade) — 77 total content errors across 8 red crates, 4m 47s combined
- `cargo test --workspace --doc` — 462 passed / 0 failed / 210 ignored, 1m 10s
- `bash scripts/check-public-api-examples.sh` (gate) and `--list` — 101-item live derivation, 19
  MISSING/SINGULAR, exit 1, 6.6s
- `bash scripts/check-doc-examples.sh` — all green (0 checked/616 skipped at Layer 2, both compile
  layers pass), 38.6s
- `bash scripts/check-doc-config.sh` — 154 YAML blocks, 0 failed, 0.9s
- `mdbook-mermaid install docs/` then `git status --porcelain -- docs` — clean before and after
- `mdbook build docs/` (with linkcheck) — "No broken links found", 4.3s
- `cargo build --examples --offline` + the 3 feature-gated invocations + `live_vendor_smoke` — all
  5 green, `--offline` works, combined ~4m
- `find examples -name '*.rs' | wc -l` — 48 (vs `ci.yml:538`'s stale "47" comment)
- `find docs/src -name '*.md' | wc -l` + SUMMARY.md cross-reference — 93 files, 0 real orphans
- `git tag -l v0.9.0` — exists locally; `git diff v0.9.0..HEAD --stat -- .project/current-exports.txt`
  — 4,376 net additions
- `awk '/^## \[0\.10\.0\]/,/^## \[0\.9\.0\]/' CHANGELOG.md` — 427 lines, 6 subsections
- `grep -n '^## 9\.' MIGRATION.md` — 722 total lines, 8 subsections

### Secondary (MEDIUM confidence — read from the repo's own prior-phase artifacts, not independently re-derived)
- `.planning/milestones/v0.8.0-phases/16-documentation-currency-the-architecture-gap/16-DOCS-01-VERDICTS.md`
  — the 8-signal-class method and row format (D-07's basis)
- `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` — the evidence-table format
  (D-02's basis)
- `.planning/phases/31-lossless-token-accounting/31-CONTEXT.md` D-29 — the `token_count` hit list

### Tertiary (LOW confidence — none used; this research avoided WebSearch/training-data claims
entirely, since every question here was answerable by running a command in the repo)

## Metadata

**Confidence breakdown:**
- Command output shapes and counts: HIGH — every figure reproduced live this session
- mdBook/examples inventory completeness: HIGH — cross-checked by direct `find`/`grep` against
  `SUMMARY.md` and `Cargo.toml`
- Verdict/sizing recommendations (S/M/L, which finding goes to which work list): not applicable —
  out of this research's scope; D-01…D-21 already specify the method, this document supplies the
  measured inputs

**Research date:** 2026-09-17
**Valid until:** Re-run at Phase 34 plan/execution time per D-23 — any commit past HEAD `c36b7729`
invalidates the exact counts above (structure and pitfalls remain valid; numbers do not carry
past a HEAD move).
