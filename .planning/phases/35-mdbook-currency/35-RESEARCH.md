# Phase 35: mdBook Currency - Research

**Researched:** 2026-09-17
**Domain:** mdBook documentation correction against a shipped Rust workspace (60 `MB-nn` page fixes)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

All decisions D-00a through D-27 in `35-CONTEXT.md` are locked and are NOT re-litigated by this
research. Summary of the ones research had to ground with live-tree evidence (full text in
`35-CONTEXT.md`, read that file for the complete decision record — this section only restates
what the researcher was asked to verify):

- **D-00a…D-00h:** work list is `34-AUDIT.md` §5 (60 IDs), audit rows stand as measured (verified
  below — `git diff --stat` against the pinned SHA is empty), shipped tree outranks any document,
  vocabulary rule (Phase 30 D-01) applies, gate is the exact `docs.yml` sequence, upgrading.md
  already agrees with MIGRATION.md (not edited), requirement prefix is `CURR-*` (planner mints next
  numbers), runnable snippets live in `crates/doc-examples` behind `// ANCHOR:` regions.
- **D-01…D-06:** page disposition rules (correct/archive/retitle), archive-tier membership,
  correct-tier membership, PROJECT.md non-goal not reopened, MB-35 retitle-and-add ADR index page,
  MB-16 migration-guide.md correction.
- **D-07…D-10:** the superstep-engine page — path `docs/src/user-guides/superstep-engine.md`, nav
  title, `Since:` marker, scope (ENG-01…ENG-05 surface only, routing/pause/Aegis linked not
  re-explained), new `crates/doc-examples/src/superstep_engine.rs` module, four dependent pages
  gain a forward link.
- **D-11…D-16:** snippet verification bright-line rule (complete-flow → anchor; fragment →
  `rust,ignore`; appendix → corrected in place + scratch-compile-proved), `doc-examples` module
  candidate list (one per signature-level finding), the `paladin_ports` import-path fix and the LLM
  adapter relocation fix, CLI `--help` capture rule, CI job table rule, clean corrections (no new
  dated callouts).
- **D-17…D-19:** vocabulary fixes (Quartermaster word removal, Medieval-military table excerpt cut,
  `GarrisonEntry` snippet + new Core Domain Entities short entries).
- **D-20…D-22:** version/MSRV/Dockerfile/feature-flag sweep — literal `"0.10.0"`, MSRV `1.88`, real
  Dockerfile base image, feature tables regenerated from `Cargo.toml`, `mem --> llm` mermaid edge
  parity.
- **D-23…D-27:** closure table per SUMMARY, one-commit-per-page granularity, CHANGELOG
  `### Documentation` subsection, Phase 36 `doc-examples` boundary (Phase 35 only adds modules),
  Phase 34 §2 rows are the evidence base (re-run only to prove a fix).

### Claude's Discretion

- Plan count and waves (natural shape: four — MB-30+module+nav; Getting Started/User
  Guides/Architecture parallel with API Reference/Contributing/Deployment/Operations; the 25
  appendix rows (CLI family as one plan); CHANGELOG+exit greps+EVIDENCE). May merge/split so long
  as MB-30 lands before its four dependents (MB-05, MB-09, MB-11, MB-22).
- Exact archive-banner wording (must name the live source and, where one exists, the ADR) and the
  exact ADR set on the index page (confirmed below: nine ADRs).
- Archive vs correct tier for `sanctum-benchmarks.md`, `battalion-benchmarks.md`,
  `release-automation.md` and any other appendix page D-02/D-03 do not name.
- Whether the three superseded "Corrected 2026-08-24" callout removals ride in their page's commit
  or a separate small commit.
- ADR index page (D-05) nav position — last Contributing entry, or directly after the retitled
  adapter guide.

### Deferred Ideas (OUT OF SCOPE)

- Regenerating `doc-coverage-report.md` from a real measurement (archived instead; Phase 36 owns
  the rustdoc bar).
- Existing `doc-examples` anchors a page fix would like to change (Phase 36 `EX-nn` territory;
  Phase 35 records each in its own `deferred-items.md`).
- Moving ADRs into `docs/src/` as a full chapter (backlog; D-05's index page is the minimal
  version).
- A script that regenerates feature-flag tables from `Cargo.toml` (D-22 does it by hand this
  phase).
- PROJECT.md corrections (planning corpus, already in the Phase 34 deferred register).
- `make doc` / rustdoc gate in `make clean-code` (Phase 36 SC2).
- Rustdoc warnings/intra-doc links (`RD-nn`), `examples/` programs and existing `doc-examples`
  module edits (`EX-nn`) — all Phase 36.
- Release re-seal, merge, tag — Phase 37.
- Any public API change (none needed this phase).
</user_constraints>

<phase_requirements>
## Phase Requirements

No `CURR-nn` IDs exist yet for Phase 35 — CONTEXT.md D-00g assigns minting to the planner: "the
planner mints the next numbers for Phase 35 in `.planning/REQUIREMENTS.md`" following the Phase 34
precedent (first plan's first task writes them into REQUIREMENTS.md, replacing the ROADMAP "TBD"
line). `CURR-01…05` are spent on Phase 34; Phase 35 mints `CURR-06` onward.

The natural 1:1 mapping from ROADMAP Phase 35's five success criteria (verified live below) is:

| Proposed ID | Description | Research Support |
|----|-------------|------------------|
| CURR-06 | Every MB-nn row in `34-AUDIT.md` §5 closed by a page edit or new page; SUMMARY.md links each new page at the audit-assigned nav position | §5 work list re-verified below (60 rows, 1 missing + 59 stale); SUMMARY.md insertion points confirmed (MB-30 before `control-flow.md`, MB-35 in Contributing) |
| CURR-07 | `mdbook build docs/` + linkcheck backend passes zero broken links via the exact `docs.yml` sequence including `mdbook-mermaid install` | Full sequence re-run live this session — green, 3.3s build, "No broken links found", `git status --porcelain -- docs` clean after mermaid install |
| CURR-08 | No touched page names a type/function/config key/route/CLI flag the v0.10.0 tree does not export; runnable snippets compile-verified via `crates/doc-examples`; illustrative snippets marked `rust,ignore` | `check-doc-examples.sh` Layers 1/1b/2 re-run live — Layer 1 compiles clean, 0/616 blocks routed to Layer 2 compile (all skipped as illustrative), README sync clean |
| CURR-09 | Book vocabulary matches the three ubiquitous-language lists — no `Quartermaster`, no bare token total beside a `TokenUsage` split | `grep -rniE '\bQuartermaster\b' docs/src` currently returns exactly one hit (commissary.md:7, the MB-02 target); Phase 31 D-29 hit list re-checked at Phase 34, only `domain-model.md` failed (Garrison field, not a bare total) |
| CURR-10 | `CHANGELOG.md` `[0.10.0]` carries a Documentation entry summarising pages added/corrected | 0.5.0 precedent line located (~line 1062); heading does not yet exist for 0.10.0 — this phase creates it |

The planner should confirm this 1:1 mapping against ROADMAP Phase 35's exact SC wording before
minting, but no alternative grouping was found necessary during research.
</phase_requirements>

## Summary

Phase 35 is a **correction phase, not a discovery phase** — 34-AUDIT.md already did the discovery
work and this research's job was to ground every CONTEXT.md decision in live-tree evidence so the
planner can write exact task instructions without re-deriving facts. All ten orchestrator research
questions were answered directly against the tree this session (HEAD `319de936`, same working tree
Phase 34 measured at `ee1fb160f8e743e638b32beb6c4e32be4ede9325` — `git diff --stat` against that SHA
for non-`.planning` paths is empty, confirmed again this session, so the D-00b re-run rule is not
triggered).

Three corrections to CONTEXT.md's own canonical-refs prose surfaced during verification, all minor
and all resolvable without reopening any decision: (1) the superstep engine's `WarEngine`/`WarGraph`
live in `crates/paladin-battalion/src/engine/`, not `paladin-core` — `paladin-core` holds only the
state types (`Battlefield`, `Waypoint`) the engine operates on; (2) `WaypointRetentionService` lives
in the facade's `src/application/services/waypoint_retention.rs`, not `paladin-core`; it wraps a
`prune()` free function in `paladin-storage/src/waypoint/retention.rs`; (3) `Vanguard` is not a
named struct — it is the `Vec<NodeId>` `vanguard` field on `Waypoint` (`waypoint.rs:675`), referred
to as "the Vanguard" only in prose/rustdoc. None of these change what the CONTEXT.md decisions say
to do; they only correct which file path a plan task should point at.

The `docs.yml` gate was re-run live end-to-end this session and is green: `mdbook build docs/`
(3.3s, "No broken links found"), `check-doc-examples.sh` (Layer 1 compiles clean, README sync
clean, Layer 2 finds 0 checked / 616 skipped — matching CONTEXT.md's "368 `rust,ignore` + 248 bare"
= 616 total block count exactly), `check-doc-config.sh` (154 YAML blocks, 0 failed). All three
pinned tool versions (`mdbook 0.4.40`, `mdbook-mermaid 0.13.0`, `mdbook-linkcheck 0.7.7`) are
already installed locally at the exact `docs.yml` pins. `mdbook-mermaid install docs/` produces a
clean `git status --porcelain -- docs` (no drift).

The D-13 import-path fix is confirmed exactly as CONTEXT.md predicted: `paladin::paladin_ports::…`
does not compile (`paladin_ports` is a sibling workspace crate, never re-exported through the
facade's `pub mod`/`pub use` surface) — the fix is the direct crate path `paladin_ports::output::…`,
used unprefixed throughout the facade's own source (`src/bin/paladin-server.rs`,
`src/config/agent_runtime.rs`, etc.). The relocated LLM adapters resolve to
`paladin_llm::openai::OpenAIAdapter`, `paladin_llm::anthropic::AnthropicAdapter`,
`paladin_llm::deepseek::DeepSeekAdapter` — note the struct is spelled `OpenAIAdapter` (capital
I-A), not `OpenAiAdapter`; `sentinel.md`'s current casing is itself part of the defect.

The nine ADRs for the D-05 index page are confirmed by directory listing and one-line decision
extraction: 0033 (cargo-doc bar), 0037 (`/v1` route surface), 0039 (HTTP topology, no
Garrison/Arsenal), 0042 (LLM-native tool calling deferred), 0047 (architecture appendix
disposition), 0048 (`paladin-eval` composition crate), 0049 (Commissary design/rename), 0050
(Treasurer reservation), 0051 (token-economy versioning/X-03 supersession).

The CLI `--help` captures for `council`, `muster`, `onboarding`, `setup-check` and the top level
were run against a locally-built `target/debug/paladin-cli` (built successfully with
`cargo build --features cli --bin paladin-cli`, ~66s cold) and confirm CONTEXT.md's fabricated-flag
claim: the real global flags are `--quiet`/`--verbose` (long form only — no `-v`/`-q` short
aliases, no `--json`), and none of `--mode`, `--synthesize`, `PALADIN_ENV_FILE`,
`PALADIN_SKIP_VALIDATION` appear anywhere in the real output.

**Primary recommendation:** Plan Phase 35 exactly along the four-wave shape CONTEXT.md's Claude's
Discretion section proposes, using the corrected file paths this research surfaces for the
superstep-engine page. Every fix is a page edit against tree evidence that has now been verified
twice (Phase 34's audit run, and this session's independent spot-checks) — there is no remaining
"go find out" work, only "go write the correction" work.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| mdBook page content correction | Docs/Content (docs/src/) | — | Prose and fenced-code correction against shipped API; no runtime tier involved |
| Runnable snippet compile-verification | Build tooling (`crates/doc-examples` + CI) | Docs/Content | New/edited `#[cfg]`-free Rust modules compiled by `cargo check -p paladin-doc-examples`; consumed by docs via `{{#include}}` |
| CLI `--help` capture | CLI binary (`src/bin/paladin-cli.rs`, `src/application/cli/`) | Docs/Content | Docs are a passive mirror of the binary's real `clap` output — no doc-side logic |
| Feature-flag / crate-map tables | Build config (`Cargo.toml` `[features]`) | Docs/Content | Docs regenerated from the authoritative `[features]` tables, never hand-maintained independently |
| ADR index page | Planning corpus (`.planning/decisions/`) | Docs/Content | New page is a read-only table of links into `.planning/decisions/`; linkcheck exempted via `follow-web-links = false` GitHub-blob-URL pattern |
| CI/CD job table | CI config (`.github/workflows/*.yml`, `protect-main-branch.json`) | Docs/Content | Docs mirror real job names/required-vs-advisory status; no independent CI logic in docs |

## Standard Stack

This phase adds no new runtime dependency and no new crate beyond what `crates/doc-examples`
already has. "Stack" here means the documentation tooling already pinned in `docs.yml`.

### Core (already pinned, re-verified installed and functional this session)

| Tool | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `mdbook` | 0.4.40 | Book build | `docs.yml` pin — [VERIFIED: local `mdbook --version` = `mdbook v0.4.40`] |
| `mdbook-mermaid` | 0.13.0 | Mermaid diagram rendering | `docs.yml` pin — [VERIFIED: local `mdbook-mermaid --version` = `mdbook-mermaid 0.13.0`] |
| `mdbook-linkcheck` | 0.7.7 | Linkcheck backend (`warning-policy = "error"`) | `docs.yml` pin — [VERIFIED: local `mdbook-linkcheck --version` = `mdbook-linkcheck 0.7.7`] |

### Supporting

| Tool | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rustfmt` | workspace toolchain | Layer 2 syntax-check of bare fenced blocks | Invoked internally by `check-doc-examples.sh`; not a separate install step |
| `pyyaml` (Python) | any | `check-doc-config.sh` YAML fence validation | [VERIFIED: already importable in this devcontainer — script ran without a separate `pip install` step; `pip` itself is not on PATH, but the module resolves] |

### Alternatives Considered

None — this phase edits prose and existing tooling; no new library selection is in scope (D-00a).

**Installation:** No install step needed in this devcontainer; all three mdBook tools and the YAML
validator are already present at the exact `docs.yml` pins. CI installs them fresh via `cargo
install … --locked` per the workflow file — the plan's `35-EVIDENCE.md` should record both the
local pre-installed state and quote the CI install commands verbatim (D-00e).

## Package Legitimacy Audit

Not applicable — this phase installs no new external package. `crates/doc-examples` gains new
internal modules only (D-09, D-12), and `Package Legitimacy Gate` protocol packages are all
already-approved workspace-internal path dependencies (`paladin-core`, `paladin-ports`,
`paladin-battalion`, `paladin-llm`, `paladin-storage` — all first-party crates, not registry
packages).

## Architecture Patterns

### System Architecture Diagram

```
34-AUDIT.md §5 (60 MB-nn rows, input)
        │
        ▼
 ┌─────────────────────────────────────────────┐
 │  Page-fix loop, one MB-nn row at a time      │
 │                                               │
 │  read §2 finding + Cites cell                │
 │        │                                     │
 │        ▼                                     │
 │  disposition? ── correct ──► edit page prose │
 │        │                     against tree    │
 │        │                     evidence (D-00c)│
 │        ├── archive ──► ADR-0047 banner +     │
 │        │               one-line correction   │
 │        └── retitle ──► nav rename + new page │
 │                                               │
 │  code block? ── complete flow ──► new/edited │
 │        │                          doc-examples│
 │        │                          module,     │
 │        │                          {{#include}}│
 │        └── fragment ──► correct + `rust,ignore`│
 └───────────────┬───────────────────────────────┘
                  │  one commit per page (D-24)
                  ▼
        docs/src/SUMMARY.md (nav insertions: MB-30, MB-35)
        crates/doc-examples/src/lib.rs (new pub mod registrations)
                  │
                  ▼
        docs.yml gate: mdbook-mermaid install → mdbook build (+linkcheck)
        → check-doc-examples.sh → check-doc-config.sh
                  │
                  ▼
        CHANGELOG.md [0.10.0] ### Documentation (created, D-25)
        35-EVIDENCE.md (closure table + final gate run, D-23)
```

### Recommended Project Structure

No new top-level structure — this phase edits within the existing tree:

```
docs/src/
├── user-guides/superstep-engine.md   # NEW (MB-30), nav-inserted before control-flow.md
├── contributing/adr-index.md          # NEW (MB-35), nav-inserted after the retitled adapter guide
├── (59 other existing pages)          # EDITED per §2 findings
└── SUMMARY.md                         # 2 insertions, 1 retitle

crates/doc-examples/src/
├── superstep_engine.rs                # NEW — WarGraph build/limits/run/waypoint-history anchors
├── (0-7 other candidate modules)      # NEW, one per D-12 signature-level page (see below)
└── lib.rs                             # pub mod registrations added, nothing else touched (D-26)

CHANGELOG.md                           # [0.10.0] ### Documentation subsection created (D-25)
.planning/phases/35-mdbook-currency/
├── 35-EVIDENCE.md                     # final plan: closure table, gate run, exit greps
└── deferred-items.md                  # phase-local, only if a current-tier defect is noticed in passing
```

### Pattern 1: Disposition-first page editing (D-01)

**What:** Before touching prose, classify the page correct / archive / retitle. The disposition
determines the edit shape, not the other way around.
**When to use:** Every one of the 60 `MB-nn` rows.
**Example (archive banner, adapted from the live ADR-0047 precedent):**
```markdown
<!-- Source: docs/src/appendix/design-and-architecture.md lines 3-9 (live, re-read this session) -->
> **Archived — historical document.** This page records <what it was> as of <date/milestone> and
> is not maintained. For the current, maintained <subject>, see <live page>. See <ADR/audit ref>
> for the disposition record.
```

### Pattern 2: Compile-verified snippet via `doc-examples` anchor (D-11a, D-09)

**What:** A complete flow or constructor/method call against the live API is moved into a
`crates/doc-examples/src/<page>.rs` module behind `// ANCHOR: name` / `// ANCHOR_END: name`,
pulled into the page with `{{#include ../../../crates/doc-examples/src/<mod>.rs:<anchor>}}`.
**When to use:** Getting Started, User Guides, Architecture, Deployment Topologies pages with a
signature-level finding (D-12's candidate list).
**Example (the exact pattern the new engine page follows — real code, verified this session):**
```rust
// Source: examples/war_engine_memory_baseline.rs (live tree, real public-API usage —
// use this file as the narrative template for crates/doc-examples/src/superstep_engine.rs)
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
```
`crates/doc-examples/Cargo.toml` already depends on `paladin-battalion`, `paladin-core`,
`paladin-ports` and `paladin-storage` (the last via its `redis-queue` feature line, but
`paladin_storage::waypoint` is NOT feature-gated — `pub mod waypoint;` is unconditional in
`crates/paladin-storage/src/lib.rs:24` — so `InMemoryWaypointStore` is reachable today with zero
`Cargo.toml` change).

### Pattern 3: Fragment correction, fenced `rust,ignore` (D-11b)

**What:** Struct/trait excerpts, partial builder chains, config shapes — corrected to match the
tree, fenced `rust,ignore`, page gains the illustrative-fragments header note if it lacks one.
**Example (the exact wording to reuse verbatim, read live this session):**
```
> Every code example targets the current **v0.8.0** workspace. The substantive examples are real,
> compiled code pulled from the `paladin-doc-examples` crate via mdBook `{{#include}}`, so they are
> checked against the live API; a few illustrative fragments are marked `rust,ignore`. The API forms
> are verified against `crates/paladin-battalion/` and `crates/paladin-ports/`.
```
(`orchestration.md` line 16 — note the version number in this sentence itself needs bumping to
v0.10.0 per D-20; `content-processing.md` line 11 carries the same pattern with a slightly
different second sentence, also v0.5.0-stale.)

### Anti-Patterns to Avoid

- **Fixing a `current`-verdict page while editing a neighbour:** D-27 forbids silent fixes to pages
  the audit already settled `current` — record the observation in the phase-local
  `deferred-items.md` instead, even if the fix looks trivial.
- **Inventing a `doc-examples` module for a fragment-level finding:** D-12 explicitly says drop a
  candidate module whose underlying fix is a D-11(b) fragment, not a complete flow — an unnecessary
  anchor grows `doc-examples` bounds it should not grow (D-11 rationale).
- **Editing an existing `doc-examples` anchor:** D-26 draws this line hard — Phase 35 only ADDS
  modules and `lib.rs` registrations. If a page fix wants an existing anchor changed, record it as
  a Phase 36 pointer in the phase-local `deferred-items.md`, don't touch it.
- **New dated "Corrected 2026-09-…" callouts:** D-16 — corrections are clean, no new callout noise;
  remove/rewrite superseded existing callouts instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Proving a snippet still compiles | A bespoke doctest harness | `crates/doc-examples` + `{{#include}}` (already exists, Phase 16) | The mechanism is built and CI-wired; adding a module is one `pub mod` line |
| CLI command syntax accuracy | Hand-transcribed flag tables | Live `cargo run --features cli --bin paladin-cli -- <sub> --help` capture, pasted verbatim into a ```text fence | The binary is the source of truth; transcription is exactly how the current fabricated-flag defect (`--mode`, `--synthesize`, …) happened |
| Feature-flag tables | Manually keeping docs and `Cargo.toml` in sync by memory | Regenerate straight from each crate's `[features]` block (D-22) — read the file, don't recall it | `Cargo.toml` is the only place a flag's existence and gate are authoritative |
| ADR discoverability | A prose paragraph re-explaining each ADR's content | A table of links using the `migration-guide.md` GitHub-blob-URL pattern (`follow-web-links = false`) | Linkcheck would otherwise try to fetch `.planning/` files, which aren't part of the published book tree |

**Key insight:** every "don't hand-roll" item above already has a house pattern shipped somewhere
in this repository (Phase 16's `doc-examples`, `migration-guide.md`'s link-URL trick, ADR-0047's
banner). This phase's job is finding and reusing the existing pattern, not inventing a new one —
consistent with D-00c ("shipped tree outranks any document").

## Runtime State Inventory

Not applicable — this is a documentation-correction phase (docs/src, CHANGELOG.md,
crates/doc-examples), not a rename/refactor/migration phase. No stored data, live service config,
OS-registered state, secrets, or build artifacts carry names this phase changes. (D-00c: "Nothing
under `src/` or `crates/` other than `crates/doc-examples` changes in this phase.")

## Common Pitfalls

### Pitfall 1: Trusting CONTEXT.md's file-path prose over the live tree for the engine surface

**What goes wrong:** A plan task cites `crates/paladin-core/src/platform/container/` for
`WarEngine`/`WarGraph`/`Vanguard`/`EngineConfig`/`EngineLimits`/`WaypointRetentionService`, then
the executor can't find the type there and either invents a wrong import or burns time
re-discovering what this research already settled.
**Why it happens:** CONTEXT.md's canonical_refs section groups "the engine surface for MB-30"
under one `paladin-core` bullet as a discovery pointer ("located by the researcher"), not as a
verified path — the actual surface spans two crates.
**How to avoid:** Use this research's confirmed split: `WarEngine`, `WarGraph`, `EngineLimits`,
`EngineError` (incl. `RecursionLimitExceeded`) → `crates/paladin-battalion/src/engine/{mod.rs,
graph.rs}`; `Battlefield`, `Waypoint` (incl. the `vanguard: Vec<NodeId>` field and
`GRAPH_FINGERPRINT_VERSION`) → `crates/paladin-core/src/platform/container/{battlefield.rs,
waypoint.rs}`; the three `WaypointPort` backends → `crates/paladin-storage/src/waypoint/{in_memory,
sqlite, postgres}.rs`; `WaypointRetentionService` (app-layer wrapper around `retention::prune`) →
`src/application/services/waypoint_retention.rs`; the app-facing `EngineConfig` (distinct from
`EngineLimits`, converts via `impl From<EngineConfig> for EngineLimits`) → `src/config/engine.rs`.
**Warning signs:** `grep -rn "pub struct WarEngine" crates/paladin-core` returns nothing (it's in
`paladin-battalion`).

### Pitfall 2: Assuming `paladin::paladin_ports::…` might work in *some* context

**What goes wrong:** A partial fix keeps the `paladin::` prefix on some pages "because other
`paladin::core::…` re-exports work," reasoning by analogy instead of checking.
**Why it happens:** The facade genuinely does re-export `core::…` and
`application::services::…` as compat paths (confirmed: `pub mod core;`,
`pub use application::services::…` exist in `src/lib.rs`) — but `paladin_ports` has no such
re-export anywhere in `src/lib.rs`. The compat pattern does not generalize.
**How to avoid:** `paladin_ports` is always imported as its own top-level crate name
(`paladin_ports::output::…`), never through the `paladin` facade, on every one of the ~15 in-tree
call sites checked this session (`src/bin/paladin-server.rs`, `src/config/agent_runtime.rs`,
`src/config/user_config.rs`, `src/infrastructure/mod.rs`). Apply the same rule on every one of
D-13's five named pages, with no page-by-page exception.
**Warning signs:** `grep -n "pub use paladin_ports\|paladin_ports::" src/lib.rs` returns only a
comment reference, never a `pub use`.

### Pitfall 3: Mis-transcribing `OpenAIAdapter` as `OpenAiAdapter`

**What goes wrong:** The fixed `sentinel.md` still reads `OpenAiAdapter` (lowercase i-a) because
that's literally what the stale page already says, and a fast pass corrects only the module path
(`infrastructure::adapters::llm::` → `paladin_llm::openai::`) without checking the type name too.
**Why it happens:** `sentinel.md` currently has both defects in the same lines
(`docs/src/appendix/sentinel.md:74,188,216`) — the wrong module path AND the wrong casing — so
fixing only the visible "big" defect (module path) leaves the casing bug uncaught by a quick read.
**How to avoid:** The real struct name, confirmed live (`crates/paladin-llm/src/openai/adapter.rs:280`),
is `OpenAIAdapter` — capital I, capital A, no lowercase "ai". Diff every corrected import line
against `grep -n "pub struct OpenAIAdapter\|pub struct AnthropicAdapter\|pub struct DeepSeekAdapter"
crates/paladin-llm/src/{openai,anthropic,deepseek}/adapter.rs` before committing the page.
**Warning signs:** `grep -n 'OpenAiAdapter' docs/src` still returns a hit after the "fix" commit.

### Pitfall 4: Forgetting the `WaypointPort` feature gate is a non-issue, but the `doc-examples` Cargo.toml dependency line still needs checking

**What goes wrong:** Assuming `InMemoryWaypointStore` needs a new `Cargo.toml` feature line on the
`paladin-storage` dependency because other waypoint backends (`sqlite`, `postgres`) are
feature-gated.
**Why it happens:** `paladin-storage/src/lib.rs` gates the `sqlite`/`mysql`/`postgres` submodules
behind `#[cfg(feature = …)]`, but NOT the `waypoint` module itself (`pub mod waypoint;` at line 24
has no `#[cfg]`) — an easy pattern to over-generalize from the neighbouring feature-gated lines.
**How to avoid:** `crates/doc-examples/Cargo.toml`'s existing `paladin-storage = { …, features =
["redis-queue"] }` line already compiles `InMemoryWaypointStore` with zero change — verified this
session by confirming `examples/war_engine_memory_baseline.rs` (a workspace example with the same
dependency shape) already imports it successfully.
**Warning signs:** A `cargo check -p paladin-doc-examples` failure citing a missing `waypoint`
module would be the actual signal something changed — not present as of this session.

## Code Examples

### The superstep-engine module template (D-09's narrative shape)

`examples/war_engine_memory_baseline.rs` (303 lines, live, currently building in CI's `examples`
job) is the closest existing analogue to what `crates/doc-examples/src/superstep_engine.rs` needs
to demonstrate, and follows exactly the "build the graph, set limits, run, inspect the waypoints"
narrative D-09/Specific Ideas calls for:

```rust
// Source: examples/war_engine_memory_baseline.rs (verified live, lines 39-49)
use paladin_battalion::engine::RunOutcome;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
```
This is a full, currently-compiling working program at the same dependency depth `doc-examples`
already has — use it as the "shape correctness" cross-check when the new module is written, not
as source to copy verbatim (it demonstrates a memory-benchmark harness, not the tutorial narrative
D-09 wants; the narrative shape should mirror `fault_tolerance.rs`'s eleven small `// ANCHOR:`
regions instead).

### `fault_tolerance.rs` anchor pattern (the house convention every new module follows)

```rust
// Source: crates/doc-examples/src/fault_tolerance.rs (verified live, header comment + first anchor)
//! Every `// ANCHOR:` region below is pulled into the Aegis user guide via
// ANCHOR: attach
... (real, compiling code) ...
// ANCHOR_END: attach
```
11 anchors total in this 374-line file (`attach`, `transience`, `retry`, `custom_predicate`,
`timeout`, `heartbeat`, `handlers`, `compensation`, `custom_handler`, `fallback`, `cache`) — the
scale reference for how many anchors a comparable single-subsystem page needs.

### CLI `--help` captures (live, this session — reusable verbatim in the CLI-family plan)

```text
$ paladin-cli --help
Paladin Multi-Agent Orchestration CLI

Usage: paladin-cli [OPTIONS] <COMMAND>

Commands:
  agent        Paladin agent operations (create, run)
  battalion    Battalion multi-agent operations (create, run)
  arsenal      Arsenal tool management (list, test)
  maneuver     Maneuver flow DSL operations (visualize, validate, execute)
  onboarding   Interactive onboarding wizard for initial setup
  setup-check  Check environment setup and configuration
  features     Discover available features and commands
  muster       Generate battalion configuration from task description
  eval         Evaluation harness operations (run scripted scenarios)
  graph        Graph document operations (export to Mermaid/DOT)
  run          Run/thread execution overlay operations (export to Mermaid)
  council      Run a council discussion
  help         Print this message or the help of the given subcommand(s)

Options:
      --quiet    Enable quiet mode (minimal output)
      --verbose  Enable verbose mode (detailed output)
  -h, --help     Print help
  -V, --version  Print version
```

```text
$ paladin-cli council --help
Run a council discussion

Usage: paladin-cli council [OPTIONS]

Options:
      --topic <TOPIC>                Discussion topic
      --participants <PARTICIPANTS>  Number of participants (2-10) [default: 3]
      --roles <ROLES>                Custom roles (comma-separated)
      --max-rounds <MAX_ROUNDS>      Maximum discussion rounds [default: 5]
      --save <SAVE>                  Save transcript to file
      --model <MODEL>                Model to use
      --temperature <TEMPERATURE>    Temperature setting
      --quiet                        Enable quiet mode (minimal output)
      --verbose                      Enable verbose mode (detailed output)
  -h, --help                         Print help
```

```text
$ paladin-cli muster --help
Generate battalion configuration from task description

Usage: paladin-cli muster [OPTIONS]

Options:
      --task <TASK>          Task description
  -o, --output <OUTPUT>      Output file path
      --execute              Execute immediately after generation
      --provider <PROVIDER>  LLM provider to use
      --model <MODEL>        Model to use
      --no-review            Skip review step
      --quiet                Enable quiet mode (minimal output)
      --verbose              Enable verbose mode (detailed output)
  -h, --help                 Print help
```

```text
$ paladin-cli onboarding --help
Interactive onboarding wizard for initial setup

Usage: paladin-cli onboarding [OPTIONS]

Options:
      --quiet    Enable quiet mode (minimal output)
      --verbose  Enable verbose mode (detailed output)
  -h, --help     Print help
```

```text
$ paladin-cli setup-check --help
Check environment setup and configuration

Usage: paladin-cli setup-check [OPTIONS]

Options:
      --verbose  Show detailed diagnostic information
      --quiet    Enable quiet mode (minimal output)
  -h, --help     Print help
```

**Build note for the CLI page requirement:** `cargo build --features cli --bin paladin-cli` took
~66s cold in this devcontainer (`Finished dev profile … in 1m 06s`); the resulting binary is
`target/debug/paladin-cli`. `[[bin]] required-features = ["cli"]` is confirmed at `Cargo.toml`
lines 368/467/472 (three separate `[[bin]]`/`[[example]]` entries reference the `cli` feature).

### D-05 ADR index table (all nine rows, confirmed titles + one-line decisions, live this session)

| ADR | Title | One-line decision |
|-----|-------|--------------------|
| 0033 | One `cargo doc` bar — ratified, measured, and its residue | Precedence order settles the zero-`warning:` `cargo doc` bar as already-ratified, not newly contested |
| 0037 | The agent route surface is `/v1` | Agent API served under `/v1`; `/health`, `/ready`, `/openapi.json`, `/docs` stay unversioned |
| 0039 | HTTP-served agents carry no Garrison and no Arsenal — a permanent property of the topology | The absence is a permanent topology property, not planned/forward scope |
| 0042 | LLM-native tool calling deferred as a future capability, with a named trigger and owner | Recorded as future capability improvement, not built |
| 0047 | `docs/src/appendix/design-and-architecture.md` disposition — archived, Sentinel re-anchored, diagram clause withdrawn | Page recorded historical, superseded by `docs/src/architecture/` |
| 0048 | `paladin-eval` as a published composition crate | Classified a composition crate; ADR-0031's default-build invariant does not apply |
| 0049 | `Commissary` design, rename rationale, and rejected names | Re-ported under new vocabulary, never as `Quartermaster` |
| 0050 | `Treasurer` reserved for cross-run spend governance | Role and scope reserved for Milestone 14; not built this cycle |
| 0051 | Token-economy phases land as clean breaks inside the untagged v0.10.0 | X-03 superseded for Phases 31-33 only, on the operator's 2026-09-14 decision |

Link form to reuse (from `migration-guide.md` lines 7-14, `follow-web-links = false` in
`docs/book.toml` so linkcheck does not attempt to fetch `.planning/` targets): GitHub blob URL, not
a relative markdown link into `.planning/`.

### Facade and per-crate `[features]` tables (D-22 source of truth, captured live)

Facade (`Cargo.toml`): `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]`; explicit flags
`redis-queue`, `s3-storage`, `openai-embeddings`, `qdrant`, `integration-tests`, `live-api-tests`,
`llm-openai`, `llm-anthropic`, `llm-deepseek`, `llm-kimi`, `llm-qwen`, `llm-grok`, `llm-ollama`,
`llm-gemini`, `llm-openai-compatible`, `llm-all` (aggregate), `vision`, `content-processing`,
`web-server`, `notifications`, `storage-mysql`, `storage-postgres`, `storage` (aggregate),
`redis-cache`, `otel`, `dev-ui`, `cli` (`= ["dep:clap", "dep:dialoguer", "dep:indicatif",
"dep:console", "dep:serde_yaml", "dep:colored", "dep:comfy-table", "dep:paladin-eval", "dep:glob",
"paladin-herald/table", "paladin-herald/color"]`), `full` (aggregate of `llm-all`,
`content-processing`, `web-server`, `notifications`, `storage`, `vision`, `redis-queue`,
`s3-storage`, `openai-embeddings`, `qdrant`, `cli`).

Per-crate: `paladin-llm` — `default = ["openai", "mock"]`, plus `anthropic`, `deepseek`, `kimi`,
`qwen`, `grok`, `ollama`, `openai-compatible`, `gemini`, `vision`, `openai-embeddings`.
`paladin-storage` — `sqlite`, `mysql`, `postgres`, `s3`, `redis-queue`, `redis-cache`, `scheduler`
(no `default`). `paladin-memory` — `default = []`, `sqlite`, `qdrant`, `content-processing`.
`paladin-content` — `web-scraping`, `rss`, `news-api`, `tiktoken`, `llm` (no `default`).
`paladin-web` — `default = []`, `dev-ui` (plus semver-checks lint-suppression comments, not
feature flags). `paladin-eval` — `live` only. `paladin-herald` — `default = []`, `table`, `color`.
`paladin-notifications` — `email`, `push`, `system` (no `default`). `paladin-core`, `paladin-ports`,
`paladin-battalion`, `doc-examples` have no `[features]` section at all.

Crate count for D-22's "all eleven library crates plus the facade": `ls crates/` returns 12
directories; excluding `doc-examples` (`publish = false`, not a catalogued library) gives exactly
11 — confirmed no undercounting.

**Mermaid-edge parity (already confirmed asymmetric):** `docs/src/architecture/crate-map.md:53`
already has `mem --> llm`; `docs/src/api-reference/crate-map.md` does NOT have it (only `mem -->
core` and `mem --> ports` at lines 62-63) — the api-reference page is the one row that still needs
the edge added, not both pages equally.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Hand-transcribed CLI flag tables in appendix pages | Live `--help` capture pasted verbatim | This phase (D-14) | Eliminates the class of defect that produced `--mode`, `--synthesize`, `PALADIN_ENV_FILE` fabrications |
| Dated "Corrected 2026-08-24" inline callouts | Clean corrections, no new dated markers | This phase (D-16) | `monitoring.md`/`troubleshooting.md`'s OTel callout and `performance-tuning.md`'s "no engine benchmark" callout are both now false and must be rewritten, not preserved |
| Single `doc-coverage-report.md` snapshot | Archived behind ADR-0047-style banner pointing at the live ADR-0033 bar | This phase (D-02); a fresh regeneration deferred to after Phase 36 | A regenerated report today would be invalidated by Phase 36's own rustdoc-warning close-out within the same milestone |

**Deprecated/outdated:**
- The pre-workspace `paladin::core::…` catalogue-path style on `stable-api.md` — moves to live
  crate paths this phase (D-22).
- `paladin::infrastructure::adapters::llm::…` — the adapters relocated to `paladin_llm::` with no
  facade re-export; any doc page still using the old path names a module that does not exist.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The proposed `CURR-06…10` numbering is a clean 1:1 mapping onto ROADMAP Phase 35's five success criteria | Phase Requirements | Low — planner reads the ROADMAP text directly at mint time and can renumber trivially; no downstream artifact depends on the exact number this research proposes |
| A2 | `pip` module resolution succeeding without a visible `pip` binary means `check-doc-config.sh`'s `pip install --quiet pyyaml` step will also succeed unmodified in a fresh CI runner | Standard Stack / Validation Architecture | Low — CI's `docs.yml` runs its own `pip install --quiet pyyaml` on a fresh Ubuntu runner where `pip` is present by default; this devcontainer's PATH quirk is local-only and does not affect the gate this phase is measured against |

All other factual claims in this document were verified live against the tree this session (marked
`[VERIFIED: …]` inline, or presented as plain fact from a command whose output is shown) or are
direct reads of `35-CONTEXT.md`/`34-AUDIT.md` (marked `[CITED: …]` where the claim doesn't restate
a locked decision verbatim).

## Open Questions

1. **Exact split of the 25 appendix rows across plans within Wave 3**
   - What we know: CONTEXT.md's Claude's Discretion says "CLI family as one plan" for the 7 CLI
     pages (`cli-usage` + MB-40..46); the remaining ~18 appendix rows have no CONTEXT.md-mandated
     grouping.
   - What's unclear: Whether to further split appendix rows by disposition (archive vs correct) or
     by subject area (storage setup pages vs benchmark pages vs security-scanning).
   - Recommendation: Leave to the planner — no research finding favors one split over another; all
     25 rows are independently closable (D-01's "no ID merged away" rule prevents any grouping from
     collapsing two IDs into one fix regardless of plan boundaries).

2. **Whether `contributing-providers.md` and `feature-flags.md`'s own `paladin::infrastructure::
   adapters::llm::…` occurrences are in scope**
   - What we know: These two pages carry the identical relocated-adapter defect D-13 names for
     `provider-expansion.md`/`sentinel.md`, confirmed by grep this session
     (`feature-flags.md:302`, `contributing-providers.md:272,367`).
   - What's unclear: `feature-flags.md` is already in the §5 work list under MB-15 (a broader
     feature-flag-table finding) so its occurrence is likely already covered incidentally — but
     `contributing-providers.md` does NOT appear anywhere in the 60-row §5 list, meaning the audit
     did not flag it as `stale` for this defect (or at all).
   - Recommendation: When fixing MB-15 (`feature-flags.md`), also fix its
     `paladin::infrastructure::adapters::llm::` occurrence as part of that row's own closure (it's
     the same page, same commit). Leave `contributing-providers.md` untouched and do NOT silently
     fix it — per D-27, record it in the phase-local `deferred-items.md` as an observed defect on a
     page the audit did not route to Phase 35, with a proposed classification (likely a missed
     `MB-nn` candidate Phase 34 should have caught, or genuinely out of the audit's declared scope
     if `contributing-providers.md` was swept and found `current` on every OTHER signal — the
     planner/executor should check the audit's §2 row for this specific page before deciding).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `mdbook` | `docs.yml` build step, SC2 | ✓ | 0.4.40 (exact pin match) | — |
| `mdbook-mermaid` | `docs.yml` install step | ✓ | 0.13.0 (exact pin match) | — |
| `mdbook-linkcheck` | `docs.yml` linkcheck backend | ✓ | 0.7.7 (exact pin match) | — |
| `rustfmt` | `check-doc-examples.sh` Layer 2 | ✓ | workspace toolchain | — |
| Python `yaml` module | `check-doc-config.sh` | ✓ (import succeeds) | unpinned | `pip` binary itself not on PATH in this devcontainer, but the module resolves without a fresh install — CI installs its own copy regardless |
| `cargo build --features cli --bin paladin-cli` | D-14 CLI `--help` captures | ✓ | workspace toolchain, ~66s cold build | — |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:** none — everything needed is present.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | mdBook build + linkcheck backend, `check-doc-examples.sh` (two-layer: `cargo check` + syntax scan), `check-doc-config.sh` (YAML fence parse) — no traditional unit-test framework applies to prose pages |
| Config file | `docs/book.toml` (`[output.linkcheck] warning-policy = "error"`, `follow-web-links = false`) |
| Quick run command | `mdbook build docs/` (3.3s measured this session) |
| Full suite command | `mdbook-mermaid install docs/ && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh` (the exact `docs.yml` sequence) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CURR-06 | Every MB-nn row closed, SUMMARY.md nav correct | manual + automated | `grep -c "MB-" <(git log --oneline --grep 'MB-')` should reach 60 unique IDs (D-24); nav position checked by `mdbook build` succeeding (broken nav = build failure) | ✅ |
| CURR-07 | `mdbook build` + linkcheck green | automated | `mdbook build docs/` (exit 0, "No broken links found" in output) | ✅ |
| CURR-08 | No stale type/fn/flag names; runnable snippets compile | automated | `./scripts/check-doc-examples.sh` (Layer 1 `cargo check -p paladin-doc-examples`, Layer 2 inline scan) | ✅ |
| CURR-09 | Vocabulary exit greps empty/allowlisted | automated | `grep -rniE '\bQuartermaster\b' docs/src` (must be empty after MB-02 fix); `grep -rn 'paladin::paladin_ports::' docs/src` and `grep -rn 'paladin::infrastructure::adapters::llm::' docs/src` (both empty after D-13 fixes) | ✅ |
| CURR-10 | CHANGELOG `[0.10.0]` Documentation subsection exists | manual (content review) | `grep -A 5 '^### Documentation' CHANGELOG.md` after the phase's final commit | ❌ (heading doesn't exist yet — created this phase, Wave 4) |

### Sampling Rate

- **Per task commit:** `mdbook build docs/` (fast, 3.3s) as a smoke check after any page edit;
  full `./scripts/check-doc-examples.sh` after any `doc-examples` module edit.
- **Per wave merge:** the full `docs.yml` sequence (`mdbook-mermaid install` → `mdbook build` →
  `check-doc-examples.sh` → `check-doc-config.sh`).
- **Phase gate:** full sequence green, plus the D-21 exit greps, plus `make api-surface`, all
  recorded verbatim in `35-EVIDENCE.md` (D-23).

### Wave 0 Gaps

None — existing test infrastructure (`docs.yml`'s own gate) covers every phase requirement. No new
test file or fixture is needed; this phase's "tests" ARE the mdBook build, the doc-examples compile
gate, and the exit greps, all already wired into CI (D-00e) and independently re-run green in this
research session.

## Security Domain

Not applicable in the ASVS sense — this phase makes no code change to any runtime attack surface
(no new endpoint, no new input parser, no new credential handling). `security_enforcement` is not
explicitly disabled in `.planning/config.json`, but the phase's own domain (prose + doc-examples
compile-time-only code) has no runtime attack surface to threat-model. The one adjacent concern —
`security-scanning.md` (MB-57) must state the Snyk-removal and CodeQL-advisory-only wording
correctly — is a documentation-accuracy concern already covered by D-15, not a new security control.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth-surface change |
| V3 Session Management | no | No session-surface change |
| V4 Access Control | no | No access-control change |
| V5 Input Validation | no | No new input parser (docs prose + compile-time-only Rust examples) |
| V6 Cryptography | no | No crypto code touched |

### Known Threat Patterns for this stack

None applicable — no new runtime code path.

## Sources

### Primary (HIGH confidence — live tree, this session)

- Direct file reads and greps against HEAD `319de936` for every path cited above: `src/lib.rs`,
  `crates/paladin-ports/src/lib.rs`, `crates/paladin-llm/src/lib.rs`,
  `crates/paladin-battalion/src/engine/{mod.rs,graph.rs,test_support.rs}`,
  `crates/paladin-core/src/platform/container/{waypoint.rs,battlefield.rs,garrison.rs,arsenal/core.rs,herald.rs}`,
  `src/config/engine.rs`, `src/config/waypoint_retention.rs`,
  `src/application/services/waypoint_retention.rs`,
  `crates/paladin-storage/src/waypoint/{retention.rs,in_memory.rs}`,
  `crates/doc-examples/{Cargo.toml,src/lib.rs,src/support.rs,src/fault_tolerance.rs}`,
  `examples/war_engine_memory_baseline.rs`, `.github/workflows/{ci.yml,release.yml,docs.yml,codeql.yml}`,
  `.github/rulesets/protect-main-branch.json`, `.planning/decisions/` (all 9 named ADRs),
  `Cargo.toml` (workspace + facade features), all `crates/*/Cargo.toml` `[features]` tables.
- Live command execution this session: `mdbook build docs/`, `mdbook-mermaid install docs/`,
  `./scripts/check-doc-examples.sh`, `./scripts/check-doc-config.sh`, `cargo build --features cli
  --bin paladin-cli`, `./target/debug/paladin-cli {--help,council --help,muster --help,onboarding
  --help,setup-check --help}`, `grep -rniE '\bQuartermaster\b' docs/src`, the three D-21 exit-grep
  baseline counts.
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` §5 — the 60-row work list (read in
  full this session).
- `.planning/phases/35-mdbook-currency/35-CONTEXT.md` — all decisions D-00a through D-27, canonical
  refs, code context, specifics, deferred ideas (read in full this session).

### Secondary (MEDIUM confidence)

- `.planning/STATE.md` Phase 34 close-out notes — Phase 34 plan-by-plan summary of what each mdBook
  sweep found (cross-referenced against the live §5 rows, not independently re-verified line by
  line beyond the spot-checks above).

### Tertiary (LOW confidence)

None — every claim in this document traces to a live-tree command or a direct file read from this
session, or restates a CONTEXT.md decision verbatim (locked, not re-verified by design).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all three mdBook tools confirmed installed at exact pin versions, gate
  re-run green end-to-end this session.
- Architecture: HIGH — every file path in the Architectural Responsibility Map and Code Examples
  sections was read or grepped directly this session; the two CONTEXT.md path corrections (engine
  crate location, WaypointRetentionService location) were caught by this verification, not assumed.
- Pitfalls: HIGH — all four pitfalls are grounded in a specific live grep/read this session, not
  speculative.

**Research date:** 2026-09-17
**Valid until:** Until the branch moves past HEAD `319de936` with a non-`.planning` change (per
D-00b/D-27's re-run rule — re-run the D-12/D-16 commands `34-EVIDENCE.md` used and diff against
`34-AUDIT.md` §5 before trusting these findings unchanged). Given this is a docs-only phase with no
other phase executing in parallel on `feature/phase-33`/`feature/phase-35`, expected stable for the
full duration of Phase 35's execution.
