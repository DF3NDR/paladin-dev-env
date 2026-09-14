# Phase 30: Token-Economy Vocabulary & Commissary Anchoring - Pattern Map

**Mapped:** 2026-09-14
**Files analyzed:** 11 (3 new ADRs, 1 new mdBook page, 7 modified files)
**Analogs found:** 11 / 11

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `.planning/decisions/0049-commissary-design-and-rename.md` (new) | config/doc-record | transform (record decision) | `.planning/decisions/0048-paladin-eval-composition-crate.md` | exact |
| `.planning/decisions/0050-treasurer-reservation.md` (new) | config/doc-record | transform | `.planning/decisions/0042-llm-native-tool-calling-deferred.md` (deferral-shape ADR) + `0048` heading skeleton | role-match |
| `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` (new) | config/doc-record | transform | `.planning/decisions/0048-paladin-eval-composition-crate.md` (supersession framing) | role-match |
| `docs/src/architecture/commissary.md` (new) | component/doc-page | request-response (concept + usage sketch) | `docs/src/architecture/domain-model.md` (adjacent architecture page, table + prose shape) | role-match |
| `docs/src/SUMMARY.md` (modify) | config/nav | transform | itself — insert one line in existing `# Architecture` block | exact |
| `docs/src/architecture/domain-model.md` (modify) | component/doc-page | transform | itself — extend existing "Medieval Military Naming Convention" table + add rule prose | exact |
| `docs/src/getting-started/configuration.md` (modify) | config/doc-page | transform | itself — existing Garrison/Sanctum config-table sections | exact |
| `.planning/PROJECT.md` (modify — Key Decisions + Constraints bullet) | config/doc-record | transform | itself — existing `## Key Decisions` table rows | exact |
| `CLAUDE.md` / `.github/copilot-instructions.md` (modify) | config/doc-table | transform | itself — existing "Naming Convention: Medieval Military Theme" table | exact |
| `crates/paladin-core/src/platform/container/herald.rs` (modify, rustdoc only) | model/rustdoc | transform | itself — existing `cost_estimate` field/builder/example doc comments | exact |
| `src/lib.rs` (modify, comment only) + `.project/project-management/paladin-project-plan-final.md` (modify, annotation) | utility/comment | transform | itself — existing provenance comment / example block | exact |

## Pattern Assignments

### `.planning/decisions/0049-commissary-design-and-rename.md` (config/doc-record)

**Analog:** `.planning/decisions/0048-paladin-eval-composition-crate.md`

**Heading skeleton** (verbatim order, `## ` level only — from `grep -n "^## "` on 0048):
```markdown
# ADR-0049: `Commissary` design, rename rationale, and rejected names

## Status

Accepted

**Date:** 2026-09-14

## Context

[design summary of verify_fits guard + dispense allocator, fail-loud/never-silent;
cite abandoned-branch ADR-0010 by path, do not copy wholesale]

## Decision

## Considered Options

## Code Locations

## Code Conformance

## Downstream Consumers
```

**Required-heading contract** (`.planning/decisions/PROMOTION.md:212-220`, verified this session):
`## Status`, `## Context`, `## Decision`, `## Considered Options`, `## Code Locations`,
`## Code Conformance`, `## Downstream Consumers` — in this exact order. `## Code Locations` and
`## Considered Options` **must be bulleted lists** (`adr-parser.cjs`'s `splitEntries` only yields
structured entries from bullet/numbered lines — a paragraph collapses into one opaque blob).

**`## Code Conformance` verdict pattern** — precedent from ADR-0048
(`0048-paladin-eval-composition-crate.md:126-134`): use `conforms` (not `must change`) — this ADR
records already-shipped state, instructs no code change.

**Numbering procedure** (`PROMOTION.md:60-75`, six-step Part A, verified):
1. Next free number is **0049** (index line: "Next free ADR number: 0049").
2. Author the substance into the standard heading set.
3. Set `## Code Conformance` to `conforms` (per D-03).
4. Cite the source document path — `git show
   origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`
   — in `## Code Locations` alongside shipped-code citations.
5. Update the "Next free ADR number" line to **0052** after all three ADRs land (0049, 0050,
   0051), appending a dated note matching the style at `PROMOTION.md:100-110`.
6. Add three rows to `.planning/PROJECT.md`'s `## Key Decisions` table.

**Content to carry (verified source material, do not re-fetch):**
- ADR-0010 design: `verify_fits` (pre-flight guard, never trims,
  `ContextOverflow { measured_tokens, allotted_tokens, provider }`) + `apportion`→`dispense`
  (bounded allocator, `Consignment`→`Stockpile`, every shed item recorded, "nothing dropped
  silently").
- Rejected-name list (union, dedup): `Quartermaster`, `Convoy`, `apportion`, `Provisioner`,
  `ProvisioningPlan`, `Muster`/`muster`, `provision()`, `ContextRation`, `Allocation`.
- Current shipped API (`crates/paladin-llm/src/services/commissary.rs`): `Commissary::new`,
  `Commissary::from_port`, `Commissary::verify_fits`, `Commissary::dispense`,
  `Commissary::allotted_tokens`; types `CommissaryPlan`, `Consignment`, `ConsignmentItem`,
  `DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryError`.

---

### `.planning/decisions/0050-treasurer-reservation.md` (config/doc-record)

**Analog:** same heading skeleton as 0048/0049 (`PROMOTION.md`'s required set) — content shape is
a **reservation/deferral** ADR, closest in spirit to `0042-llm-native-tool-calling-deferred.md`
(a capability recorded as future, not built, with a named reintroduction trigger).

**Reuse pattern:** name the reserved term, its future owner (Milestone 14), what it will do
(allowances, pricing, `cost_estimate` production, pacing), what it does NOT replace (`TokenBudget`
install point at `src/application/services/paladin/middleware/limits.rs`), the downstream
guardrail (`GarrisonTreasury` collision in the Web3 Security Paladin app — framework-only word),
and the rejected alternatives (`Paymaster` rejected, `Comptroller` collision-free alternative,
operator chose `Treasurer`).

**`## Code Conformance`:** `conforms` — reserves a word, institutes no code.

---

### `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` (config/doc-record)

**Analog:** `0048-paladin-eval-composition-crate.md`'s supersession-framing pattern (an ADR that
narrows/overrides an existing rule for a scoped set of future phases).

**Content:** records that v0.10.0 corpus rule X-03 (`.project/v0.10.0/00-program-overview.md:44`)
is superseded **for Phases 31-33 only** by the Milestone 13 overview §5.1 clean-break policy.
State the `MIGRATION.md` §9.2 row + `cargo semver-checks` allowlist row requirement as
documentation for the downstream refactor, never a compatibility shim.

**`## Code Conformance`:** `conforms` — this phase makes no public-API change itself; the ADR
records a forward-looking policy exception for later phases.

---

### `.planning/PROJECT.md` — Key Decisions rows (modify)

**Analog:** existing table rows at `.planning/PROJECT.md:1308-1317`.

**Row shape to mirror exactly:**
```markdown
| [Decision title](.planning/decisions/00NN-slug.md) (ADR-00NN) | <rationale, one sentence, cites concrete file:line evidence> | conforms |
```
Table header comment to preserve (`PROJECT.md:1310-1311`):
```markdown
<!-- LOCKED DECISIONS. See .planning/decisions/ for the full ADR text behind each row — this
     table links to it rather than restating it. -->
```
Add three new rows (one per ADR: 0049, 0050, 0051) below the existing last row, in ADR-number
order, following the exact `| [title](path) (ADR-NNNN) | rationale | outcome |` shape.

---

### `docs/src/architecture/domain-model.md` (modify)

**Analog:** itself — existing table at lines 6-31.

**Table row shape to mirror** (verified, `docs/src/architecture/domain-model.md:12-31`):
```markdown
| **Commissary** | Input-side, per-call window-rationing officer | `Commissary` · `crates/paladin-llm/src/services/commissary.rs` |
```
Insert alphabetically/thematically near `Citadel`/`Herald` rows (after `Citadel`, before `Herald`,
or after `Herald` — Claude's Discretion per CONTEXT.md). Table header stays:
```markdown
| Term | DDD Concept | Rust Type / Location |
|------|-------------|---------------------|
```
Add the vocabulary-rule prose as a new subsection before or after the table (file currently states
the convention as a flat mandate — CONTEXT.md/RESEARCH.md recommend adding an explicit
plain-vs-medieval split, e.g.: "Units and measures (`TokenUsage`, `max_tokens`,
`max_context_tokens`, `TokenBudget`) and technical port traits (`TokenCounterPort`, `LlmPort`,
`EmbeddingPort`) keep plain industry names. Domain roles, places and events get Medieval-Military
names.").

---

### `CLAUDE.md` / `.github/copilot-instructions.md` — Naming Convention table (modify)

**Analog:** itself — existing table (verified, 14 rows: Paladin, Battalion, Formation, Phalanx,
Campaign, Chain of Command, Commander, Garrison, Arsenal, Armament, Citadel, Herald, Armory,
Quest — in `.github/copilot-instructions.md`, the `@`-imported source table; `CLAUDE.md` carries
no separate table).

**Row shape to mirror** (from the copilot-instructions.md markdown table, `| Term | Definition |
Module Location |`):
```markdown
| **Commissary** | Input-side, per-call window-rationing officer | `crates/paladin-llm/src/services/commissary.rs` |
```
Edit `.github/copilot-instructions.md` only (source table); do not create a duplicate table in
`CLAUDE.md` — it only `@`-imports the file. Add exactly one row; do not reconcile the other six
missing rows (Sanctum/Sentinel/Conclave/Council/Grove/Maneuver) — out of scope (Pitfall 4 in
RESEARCH.md).

---

### `docs/src/architecture/commissary.md` (new mdBook page)

**Analog:** `docs/src/architecture/domain-model.md` for prose/table structure; the concrete usage
sketch must mirror `crates/paladin-llm/src/services/commissary.rs`'s own `#[cfg(test)]` tests
(lines 631-686, 985-1008) — never invented API.

**Code block to mirror, verbatim shape** (from RESEARCH.md Code Examples, sourced from
`commissary.rs` tests lines 631-686, use `rust,ignore` since `MockCounter`/
`capabilities_with_window` are test-only helpers):
```rust,ignore
use std::sync::Arc;
use paladin_llm::services::commissary::{Commissary, CommissaryPlan, Consignment, ConsignmentItem};
use paladin_ports::output::llm_port::ProviderCapabilities;

let capabilities = ProviderCapabilities {
    max_context_tokens: Some(100),
    ..Default::default()
};
let commissary = Commissary::new(
    "deepseek",
    capabilities,
    /* counter: Arc<dyn TokenCounterPort> */ counter,
    /* is_exact_counter */ false,
    CommissaryPlan::default(),
)?;

let mut consignment = Consignment::new();
consignment.push(ConsignmentItem { label: "high-priority".into(), body: "A".repeat(300), priority: 1 });
consignment.push(ConsignmentItem { label: "low-priority".into(),  body: "B".repeat(300), priority: 2 });

let stockpile = commissary.dispense("", &consignment)?;
// stockpile.shed[0].label == "low-priority"  (lower priority == shed first)
// stockpile.dispensed[0].label == "high-priority"
```
Plus the `verify_fits` pre-flight guard block (`commissary.rs:561-576`):
```rust,ignore
match commissary.verify_fits(&assembled_prompt) {
    Ok(measured_tokens) => { /* proceed to call the provider */ }
    Err(CommissaryError::ContextOverflow { measured_tokens, allotted_tokens, provider }) => {
        // fail loud — never silently truncate
    }
    Err(other) => { /* CommissaryError variants: UndeclaredContextWindow, ReservationExceedsWindow, InvalidConfig */ }
}
```
A mermaid diagram is welcome (`mdbook-mermaid` 0.13.0 installed) showing
`Consignment → Commissary::dispense → Stockpile / ShedItem`.

---

### `docs/src/SUMMARY.md` (modify)

**Analog:** itself — existing `# Architecture` block (verified, lines ~34-40):
```markdown
# Architecture

- [Overview](architecture/overview.md)
- [Hexagonal Design](architecture/hexagonal-design.md)
- [Domain Model](architecture/domain-model.md)
- [Design Patterns](architecture/design-patterns.md)
- [Crate Map](architecture/crate-map.md)
```
Insert one new line, matching the exact `- [Title](architecture/file.md)` format, e.g.:
```markdown
- [Commissary](architecture/commissary.md)
```
placed after `[Domain Model]` (natural adjacency) or after `[Design Patterns]` — planner's
discretion; must resolve under `mdbook-linkcheck` (`warning-policy = "error"`).

---

### `docs/src/getting-started/configuration.md` (modify — `max_tokens` table)

**Analog:** itself — existing Garrison/Sanctum config-table pattern (verified, lines 240-283):
each config section has a fenced `yaml` block, then a `| Key | Type | Default | Description |`
table, then an `**Env vars:**` line.

**New table shape to add** (four rows, no yaml fence needed since these are four *distinct*
existing keys, not one section):
```markdown
## Token Budget Terminology

Paladin uses `max_tokens` in four independent, non-overlapping senses:

| Meaning | Config key / type | Owner |
|---|---|---|
| Garrison store cap | `garrison.max_tokens` | `src/config/` (Garrison config) |
| RAG injection cap | `rag.max_tokens` | `src/config/` (RAG config) |
| Per-request completion cap | `LlmRequest` metadata `"max_tokens"` (OpenAI/DeepSeek fallback-override); `llm.anthropic.max_tokens` / `ANTHROPIC_MAX_TOKENS` (Anthropic, required) | provider adapters (`crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`) |
| Run-level budget cap | `agent_runtime.token_budget.max_tokens` | `src/application/services/paladin/middleware/limits.rs` |

Any future Treasurer-level cap uses a distinct key, `allowance`, never `max_tokens`.
```
Keep the existing Garrison-section inline comment `max_tokens: 4000  # Context-window token
budget` (line ~248) unchanged — it does not contradict "Garrison store cap" (Pitfall 2 in
RESEARCH.md). Do not invent a single unified `llm.max_tokens` key (Pitfall 3).

---

### `crates/paladin-core/src/platform/container/herald.rs` (modify, rustdoc only)

**Analog:** itself — existing doc comments at the four verified line ranges.

**Field-doc style to mirror** (verified, line ~521):
```rust
    /// Estimated cost in USD (based on token usage)
    pub cost_estimate: Option<f64>,
```
→ reword to:
```rust
    /// Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet.
    pub cost_estimate: Option<f64>,
```

**Doc-comment bullet list style to mirror** (verified, lines ~380-395):
```rust
/// * `cost_estimate` - Estimated cost in USD based on token usage and model pricing
```
→ reword to note reserved status, e.g.:
```rust
/// * `cost_estimate` - Reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet
```

**Builder-method doc style to mirror** (verified, ~line 607):
```rust
    /// Set the cost estimate
    pub fn cost_estimate(mut self, cost_estimate: f64) -> Self {
```
→ reword the `///` line only (do not touch the signature):
```rust
    /// Set the cost estimate (reserved for the Treasurer, Milestone 14 / FUT-08 — no in-tree producer yet)
    pub fn cost_estimate(mut self, cost_estimate: f64) -> Self {
```

**Example doc-test comment to mirror** (verified, ~line 471):
```rust
///     .cost_estimate(0.045)  // $0.045 based on GPT-4 pricing
```
→ reword the trailing comment only, e.g.:
```rust
///     .cost_estimate(0.045)  // illustrative value; field is reserved for the Treasurer (Milestone 14)
```
Do **not** remove the field, accessor, or builder method — rustdoc text only, no signature change,
no `MIGRATION.md` §9.2 entry, no semver-checks allowlist change.

---

### `src/lib.rs` (modify, comment only) / `.project/project-management/paladin-project-plan-final.md` (modify, annotation)

**Analog:** itself — the existing single-line provenance comment at `src/lib.rs:195`:
```rust
// v0.10.0-native re-port of the removed Quartermaster/Convoy/apportion capability
```
→ reword to drop the retired term while keeping the provenance note, e.g.:
```rust
// v0.10.0-native Commissary service (prompt-window budgeting: verify_fits + dispense)
```

**`.project/project-management/paladin-project-plan-final.md`** — one-line historical annotation
near the `name: "SirQuartermaster"` example (line 1112), in the surrounding prose style of that
document (do not rewrite the document, one line only), e.g. a bracketed note:
```
<!-- Historical example name; "Quartermaster" as a budgeting term was retired and replaced by
     Commissary — see .planning/decisions/0049-commissary-design-and-rename.md -->
```

## Shared Patterns

### ADR heading skeleton and numbering
**Source:** `.planning/decisions/0048-paladin-eval-composition-crate.md` + `PROMOTION.md:212-230,
60-75`
**Apply to:** all three new ADRs (0049, 0050, 0051)
```markdown
## Status
## Context
## Decision
## Considered Options   <!-- bulleted list, required for adr-parser.cjs -->
## Code Locations       <!-- bulleted list, required for adr-parser.cjs -->
## Code Conformance     <!-- `conforms` or `must change`, D-03 contract -->
## Downstream Consumers
```
All three verdicts should read `conforms` (none instructs a code change).

### Key Decisions table row
**Source:** `.planning/PROJECT.md:1308-1317`
**Apply to:** all three ADRs, as three new rows
```markdown
| [Title](.planning/decisions/00NN-slug.md) (ADR-00NN) | <rationale> | conforms |
```

### Vocabulary-list agreement (three independent lists)
**Source:** `.planning/PROJECT.md` Constraints bullet (~1236-1239), `docs/src/architecture/domain-model.md` table, `.github/copilot-instructions.md` table
**Apply to:** all three — add a `Commissary` entry to each; none is auto-generated from another,
so each edit is a separate, independent diff that must not contradict the others. `Treasurer` is
named in prose only (reserved, not a live row) per CONTEXT.md.

## No Analog Found

None — every file in scope has a close self-analog (modify-in-place) or a clear cross-file analog
(0048 for ADR shape). This is a docs-only phase; no code pattern gaps exist.

## Metadata

**Analog search scope:** `.planning/decisions/`, `docs/src/`, `.planning/PROJECT.md`, `CLAUDE.md`,
`.github/copilot-instructions.md`, `crates/paladin-core/src/platform/container/herald.rs`,
`src/lib.rs`, `.project/project-management/paladin-project-plan-final.md`
**Files scanned:** 11 target files + 5 analog/reference files (0048, PROMOTION.md, domain-model.md,
configuration.md, herald.rs, SUMMARY.md, PROJECT.md Key Decisions table)
**Pattern extraction date:** 2026-09-14
