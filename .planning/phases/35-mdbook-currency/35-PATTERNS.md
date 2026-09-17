# Phase 35: mdBook Currency - Pattern Map

**Mapped:** 2026-09-17
**Files analyzed:** ~66 (2 new pages, 1-8 new `doc-examples` modules, 59 edited pages treated as one
recurring pattern, `SUMMARY.md`, `CHANGELOG.md`, `35-EVIDENCE.md`)
**Analogs found:** 6 strong analogs covering all file classes / 66

This is a documentation-correction phase — "role" below is repurposed as *page/module kind* and
"data flow" as *edit shape* (new content / in-place correction / archive banner / index table),
since the usual controller/service/CRUD taxonomy does not apply to mdBook prose and compile-checked
doc examples.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `docs/src/user-guides/superstep-engine.md` | new guide page | complete-flow narrative + `{{#include}}` | `docs/src/user-guides/fault-tolerance.md` | exact |
| `crates/doc-examples/src/superstep_engine.rs` | new doc-examples module | compile-verified anchors | `crates/doc-examples/src/fault_tolerance.rs` | exact |
| `crates/doc-examples/src/lib.rs` | module registry | append-only edit | itself (existing `pub mod` list) | exact |
| `docs/src/contributing/adr-index.md` | new index page | link table, no prose | `docs/src/api-reference/migration-guide.md` (link-URL pattern) | role-match |
| 25 appendix pages, archive-tier (MB-01, MB-39, MB-47, MB-59, MB-60, …) | reference page | archive banner + one-line correction | `docs/src/appendix/design-and-architecture.md` (ADR-0047 banner, lines 3-9) | exact |
| 7 CLI pages (`cli-usage` + MB-40..46) | reference page | live `--help` capture replacing hand-written syntax block | none pre-existing in-tree (see "No Analog Found") — closest shape is `migration-guide.md`'s pointer style | partial |
| Guide/architecture pages with signature-level findings (MB-27, MB-20, MB-10, MB-19, MB-24, MB-28, MB-12) | reference page | fragment → anchor promotion | `docs/src/user-guides/orchestration.md` (header note) + `crates/doc-examples/src/fault_tolerance.rs` (anchor shape) | exact |
| Remaining `S`/`M` correction rows (version pins, feature tables, import paths, vocabulary) | reference page | in-place prose/table correction | `docs/src/api-reference/migration-guide.md` (Timeline correction, MB-16) | role-match |
| `docs/src/SUMMARY.md` | nav index | 2 insertions + 1 retitle | itself (existing structure) | exact |
| `CHANGELOG.md` `[0.10.0]` `### Documentation` | changelog subsection | append new heading, list bullets | `CHANGELOG.md` `[0.5.0]` `### Documentation` (line 1062) | exact |
| `.planning/phases/35-mdbook-currency/35-EVIDENCE.md` | phase evidence file | gate-run transcript + closure table | `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` and `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` | exact |

## Pattern Assignments

### `docs/src/user-guides/superstep-engine.md` (new guide page)

**Analog:** `docs/src/user-guides/fault-tolerance.md` + its module `fault_tolerance.rs`

**Since: marker** (copy verbatim form from `docs/src/api-reference/platform-api.md` lines 1-6):
```markdown
# Platform API — Runs, Threads, Assistants, Schedules, Webhooks

**Since:** v0.10.0 (PRD 06, Phase 27)
**Crates:** `paladin-web` (routes/DTOs), `paladin-ports` (port contracts), `paladin-core` (`Run`,
`RunSchedule`, `WebhookDelivery`), facade `src/application/services/run/*` (the durable worker
pool, streaming bus, schedule service, webhook delivery service)
```
→ for the new page: `**Since:** v0.10.0 (Phase 22)` per D-07, followed by a `**Crates:**` line
naming `paladin-battalion` (`WarEngine`, `WarGraph`), `paladin-core` (`Battlefield`, `Waypoint`),
`paladin-storage` (`WaypointPort` backends).

**Illustrative-fragment header note** (copy verbatim from `docs/src/user-guides/orchestration.md`
lines 14-17, bump the version per D-20):
```markdown
> Every code example targets the current **v0.8.0** workspace. The substantive examples are real,
> compiled code pulled from the `paladin-doc-examples` crate via mdBook `{{#include}}`, so they are
> checked against the live API; a few illustrative fragments are marked `rust,ignore`. The API forms
> are verified against `crates/paladin-battalion/` and `crates/paladin-ports/`.
```

**Include pattern** — mirror `fault-tolerance.md`'s narrative order (one `{{#include
../../../crates/doc-examples/src/<mod>.rs:<anchor>}}` per stage of the story: build graph → set
limits → run → inspect waypoints), matching the anchor scale of `fault_tolerance.rs` (11 small
anchors for one subsystem — D-09 wants roughly build/configure/run/inspect, i.e. 4).

---

### `crates/doc-examples/src/superstep_engine.rs` (new doc-examples module)

**Analog:** `crates/doc-examples/src/fault_tolerance.rs`

**Module header + anchor shape** (lines 1-21, copy structurally):
```rust
//! Examples for `docs/src/user-guides/fault-tolerance.md` (Phase 25, D-32).
//!
//! Every `// ANCHOR:` region below is pulled into the Aegis user guide via
//! mdBook `{{#include}}`, so a sample in the guide cannot drift from the
//! landed API: `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;
use std::time::Duration;

use crate::support::{create_paladin, mock_paladin_port};

// ANCHOR: attach
use paladin_battalion::engine::{EngineLimits, InputMapping, NodeSpec, WarGraph};
use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, TimeoutPolicy};
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::waypoint::NodeId;

/// Attach an `Aegis` to one node, with a graph-wide default for the rest.
pub fn attach_an_aegis() -> Result<WarGraph, Box<dyn std::error::Error>> {
```
Each function is one `// ANCHOR: name` ... `// ANCHOR_END: name` region returning a `Result<_, Box<dyn
std::error::Error>>`, using `crate::support::{create_paladin, mock_paladin_port}` for mock
dependencies (per D-09: `support.rs` is imported, never edited — D-26).

**Real-API import cross-check** (for shape correctness, not literal copy — from
`examples/war_engine_memory_baseline.rs` lines 39-49, verified live this session per RESEARCH.md):
```rust
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
Note per RESEARCH.md Pitfall 1: `WarEngine`/`WarGraph`/`EngineLimits` live in `paladin-battalion`,
NOT `paladin-core`; only `Battlefield`/`Waypoint` state types live in `paladin-core`.

---

### `crates/doc-examples/src/lib.rs` (module registry)

**Analog:** itself — append-only edit

**Current form** (lines 1-19, full file already in context — no re-read needed):
```rust
pub mod support;

pub mod agent_runtime;
pub mod bridge;
pub mod content;
pub mod deployment_topologies;
pub mod fault_tolerance;
pub mod http_service_host;
pub mod orchestration;
pub mod queue_worker;
pub mod readme;
pub mod sidecar;
```
Add `pub mod superstep_engine;` plus one line per D-12 candidate module actually written, keeping
alphabetical order to match the existing list's convention.

---

### `docs/src/contributing/adr-index.md` (new ADR index page)

**Analog:** `docs/src/api-reference/migration-guide.md` lines 7-14 (GitHub blob-URL link form,
`follow-web-links = false` so linkcheck never fetches it)

```markdown
This historical guide stops at v0.5.0. The v0.10.0 upgrade record lives on the
[Upgrading](upgrading.md) page and in the root
[`MIGRATION.md`](https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md) file, which
together cover every behavioral change, Rust API change, schema migration, configuration
change and the operator upgrade checklist for v0.9.x → v0.10.0.
```
→ for the ADR index page: one table row per ADR using the same blob-URL form, e.g.
`[0049](https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/0049-commissary-design-and-rename.md)`.
The nine rows and one-line decisions are already tabulated in `35-RESEARCH.md` "D-05 ADR index
table" — copy that table verbatim into the page.

**SUMMARY.md insertion point** (Contributing section, current full list, lines 77-84):
```markdown
# Contributing

- [Development Setup](contributing/development-setup.md)
- [Testing Guide](contributing/testing-guide.md)
- [Architecture Decisions](contributing/architecture-decisions.md)
- [Contributing Providers](contributing/contributing-providers.md)
- [Branching Model](contributing/branching-model.md)
```
Per D-05: retitle the `architecture-decisions.md` entry to "Adapter Development Guide" and insert a
new `[Architecture Decisions](contributing/adr-index.md)` entry directly after it (Claude's
Discretion may instead place it last in Contributing — either is a one-line nav edit).

---

### Archive-tier appendix pages (MB-01, MB-39, MB-47, MB-59, MB-60, …) (reference page, archive banner)

**Analog:** `docs/src/appendix/design-and-architecture.md` lines 3-9 — the exact ADR-0047 banner
shape to reuse verbatim, substituting the subject/link/ADR per page:
```markdown
# Paladin Framework: Design and Architecture Outline

> **Archived — historical document.** This page is superseded and is retained only as a historical
> record; it is not maintained. For the current, maintained architecture documentation, see the
> [Architecture](../architecture/overview.md) chapter — it covers Commander, Sanctum, Maneuver,
> Council, Conclave and Grove. For the Sentinel Vision System (multimodal capabilities), see
> [Sentinel](sentinel.md). This disposition, the metric re-anchoring, and the withdrawal of this
> page's originally-planned diagram clause are recorded in ADR-0047
> (`.planning/decisions/0047-architecture-appendix-disposition.md`).
```
No page is deleted; the H1 title stays, only the blockquote banner is inserted immediately after it
plus the specific one-line factual corrections the row's §5 finding cites (D-01, D-02).

---

### CLI family pages (`cli-usage` + MB-40..46) (reference page, `--help` capture replacement)

**No strong in-tree analog for the capture format itself** — this is a new correction pattern (D-14).
Use the exact captures already run live in `35-RESEARCH.md` "CLI `--help` captures" section
verbatim as the content source, in a ```text fence whose first line is the command:
```text
$ paladin-cli council --help
Run a council discussion

Usage: paladin-cli council [OPTIONS]

Options:
      --topic <TOPIC>                Discussion topic
      --participants <PARTICIPANTS>  Number of participants (2-10) [default: 3]
      ...
  -h, --help                         Print help
```
Every CLI page states the build requirement once:
`cargo build --release --features cli --bin paladin-cli` (D-14, `Cargo.toml` `[[bin]]
required-features = ["cli"]`).

---

### Fragment-level pages with signature findings (MB-27, MB-20, MB-10, MB-19, MB-24, MB-28, MB-12)

**Analog:** header note from `orchestration.md` (above) plus the `fault_tolerance.rs` anchor shape
(above) — promote the fragment into a small new `doc-examples` module per D-12, following the same
`// ANCHOR:` / `pub fn` / `crate::support::…` shape shown in the fault-tolerance excerpt.

---

### Remaining in-place correction rows (version pins, feature tables, import paths, vocabulary)

**Analog:** `docs/src/api-reference/migration-guide.md` — corrected-in-place prose, no banner, no
new module:
```markdown
# Migration Guide

This guide covers all breaking changes since v0.1.0 up to the current **v0.5.0** release.
```
→ MB-16's fix (D-06): reword to "…up to the current v0.10.0 release" and correct the Timeline row
marking `0.1.0` as Current to v0.10.0, keeping the pointer-only design (the rest of the file already
correctly points at `upgrading.md` / `MIGRATION.md`).

Import-path fix (D-13, five pages) — always `paladin_ports::output::…` as a bare crate path, never
`paladin::paladin_ports::…`; adapter paths become `paladin_llm::openai::OpenAIAdapter` /
`paladin_llm::anthropic::AnthropicAdapter` / `paladin_llm::deepseek::DeepSeekAdapter` (capital I-A —
watch the `OpenAIAdapter` casing pitfall RESEARCH.md documents).

---

### `docs/src/SUMMARY.md` (nav index)

**Analog:** itself, User Guides section (lines 12-30, already fully read above) — MB-30 inserts a
new `- [WarEngine: Battlefield State & Superstep Execution](user-guides/superstep-engine.md)` line
immediately after `- [Maneuver Flow DSL](user-guides/maneuver-flow-dsl.md)` and before
`- [Control Flow: Dynamic Routing & Subgraphs](user-guides/control-flow.md)`.

---

### `CHANGELOG.md` `[0.10.0]` `### Documentation` (changelog subsection)

**Analog:** `[0.5.0]` `### Documentation` at line 1062:
```markdown
### Documentation

- Every existing documentation file was audited (current / stale / delete), migrated into the mdBook
  chapter structure, and rewritten so that all code examples compile and all internal links resolve
  (`mdbook build` passes with linkcheck `warning-policy = "error"`, zero broken links).
```
Per D-25: place the new `### Documentation` heading after `### Fixed` and before `### Known
limitations` in the `[0.10.0]` entry; one bullet per nav section plus the new-page and archived-page
bullets; no `MB-nn` IDs in the prose.

---

### `.planning/phases/35-mdbook-currency/35-EVIDENCE.md` (phase evidence file)

**Analog:** `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` (exact commands/
captures format) and `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` (naming
convention `NN-*-EVIDENCE.md` house pattern, referenced by CONTEXT.md D-23 as "the `NN-EVIDENCE.md`
house pattern"). Content per D-23: closure table (`MB-nn | page | disposition | commit | how the §5
finding was addressed`), full `docs.yml` sequence output, `git status --porcelain -- docs` check,
linkcheck summary line, both scripts' result lines, `make api-surface` output, and the D-21 exit
greps verbatim.

## Shared Patterns

### Disposition-first classification (D-01)
**Source:** `35-CONTEXT.md` D-01/D-02/D-03
**Apply to:** all 60 `MB-nn` rows — classify correct / archive / retitle before editing; the
disposition determines which analog above applies.

### Compile-verified snippet mechanism
**Source:** `crates/doc-examples` + `scripts/check-doc-examples.sh` Layer 1 (mechanism only, no
edit needed to the script itself)
**Apply to:** every complete-flow code block on Getting Started / User Guides / Architecture /
Deployment Topologies pages (D-11a).

### Illustrative-fragment marker
**Source:** `docs/src/user-guides/orchestration.md` line 16 / `docs/src/user-guides/content-processing.md`
line 11
**Apply to:** every page that keeps a `rust,ignore` fragment and lacks the header note already
(D-11b).

### `paladin_ports` / LLM-adapter import-path fix
**Source:** live grep evidence in `35-RESEARCH.md` Pitfall 2/3 — `paladin_ports::output::…` as a
bare crate path; `paladin_llm::{openai,anthropic,deepseek}::…Adapter` (correct casing
`OpenAIAdapter`)
**Apply to:** `minio-file-repository-setup.md`, `redis-queue-adapter-setup.md`,
`sanctum-migration.md`, `port-trait-template.md`, `provider-expansion.md`, `sentinel.md`,
`feature-flags.md` (D-13).

### Archive banner
**Source:** `docs/src/appendix/design-and-architecture.md` lines 3-9
**Apply to:** `doc-coverage-report.md`, `build-baselines.md`, `user-system.md`, `user-rest-api.md`,
`contributing-legacy.md`, and any further archive-tier page Claude's Discretion assigns (D-02).

### One-commit-per-page granularity
**Source:** `35-CONTEXT.md` D-24 — `docs(35): <what changed> (MB-nn[, MB-mm])`, plain `git add --
<files> && git commit -q -m … -- <files>` with a long timeout (pre-commit clippy hook triggers on
staged `.rs`/`Cargo.toml`).
**Apply to:** every page fix and every `doc-examples` module addition.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| CLI `--help` capture blocks (7 pages) | reference page | live-capture replacing hand-written syntax | No existing docs page currently uses a live-captured `--help` block; this is a new correction pattern introduced by D-14. Use the verbatim captures already run in `35-RESEARCH.md` as the content source instead of an in-tree analog. |
| CI job table (`deployment/cicd.md`, `contributing/testing-guide.md`) | reference page | table derived from `.github/workflows/*.yml` | No existing docs page tabulates CI jobs this way; D-15 specifies the table shape directly (job / what it gates / required-or-advisory) — build from `.github/workflows/ci.yml`, `release.yml`, `.github/rulesets/protect-main-branch.json`, not from a doc analog. |

## Metadata

**Analog search scope:** `docs/src/` (all chapters), `crates/doc-examples/src/`, `CHANGELOG.md`,
`.planning/phases/33-*/`, `.planning/phases/34-*/`, `examples/war_engine_memory_baseline.rs`.
**Files scanned:** ~20 (targeted reads/greps only; large 60-row work list treated as recurring
pattern classes per CONTEXT.md/RESEARCH.md rather than read file-by-file — RESEARCH.md already
supplies the per-page §2 findings and Cites cells that drive each edit).
**Pattern extraction date:** 2026-09-17
