---
phase: 30
slug: token-economy-vocabulary-commissary-anchoring
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-14
---

# Phase 30 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — all three `30-0N-PLAN.md` files carry a `<threat_model>` block (10 distinct threat ids plus the shared `T-30-SC` package-install item; the cross-cutting `T-30-01` and `T-30-03` recur in every plan and are recorded once each below with the union of their evidence). No `30-0N-SUMMARY.md` carries a `## Threat Flags` section — the phase is documentation only: three ADRs, one mdBook page, two doc tables, five rustdoc lines and two comment edits, with no dependency, client, endpoint or external call added. Verification depth: ASVS L1 grep-depth, per the short-circuit rule (`threats_open: 0`, `register_authored_at_plan_time: true`, `asvs_level: 1`); the anti-invention gate (T-30-02) and the phase-wide `.rs`/manifest diff guard (T-30-03, T-30-SC, T-30-10) were additionally executed locally from the plan `<verify>` bodies rather than only grepped.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Abandoned branch history → live decision record | Text read from `origin/feature/quartermaster-prompt-budgeting` crosses into `.planning/decisions/`, where readers treat it as current; provenance must survive the crossing | Historical ADR prose; branch-qualified citations |
| Planning corpus → published documentation | `docs/src/**` is built and published by `docs.yml`; anything written there reaches consumers outside this repo | mdBook page, configuration table |
| Shipped crate API → documentation prose | The mdBook page asserts what the crate offers; overstated API misleads a downstream integrator | Method and type names |
| Operator conversation → durable policy record | A verbally granted exception becomes a citable ADR later agents execute against; over-broad wording grants more than asked | ADR-0051 scope clause |
| Framework vocabulary → downstream app vocabulary | The reserved officer name crosses into another repo's domain language where an adjacent fixture already exists | Reserved term `Treasurer`; guardrail `GarrisonTreasury` |
| Planning record → future executor behaviour | Phases 31-33 read ADR-0051 as authorization to remove public API | Supersession scope (phase numbers) |
| Documentation phase → compiled crate | The only plan opening a `.rs` file must write comment text only, never code | `///` and `//` lines in `herald.rs`, `src/lib.rs` |
| rustdoc → published API docs | `herald.rs` doc text is published by `cargo doc`; a wrong claim about a field's producer misleads every downstream reader | `cost_estimate` reservation note |
| Configuration guide → operator behaviour | A table row naming a nonexistent config key makes an operator write a setting that is silently ignored | `max_tokens` key paths |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-30-01 | Information disclosure | New ADR, mdBook page, configuration table and rustdoc text | low | mitigate | Credential-shape grep (`sk-…`, `AKIA…`, `api_key=<value>`, `Bearer <token>`, `*_API_KEY=<value>`) over every non-`.planning/phases` file changed in the phase (`8ed14aea..HEAD`, 23 files): 0 real hits — the only matches are pre-existing `${OPENAI_API_KEY}`-style env-var placeholders in `.github/copilot-instructions.md` (the phase diff on that file is the single Commissary vocabulary row) and substring false positives (`task-completion`, `risk-accepted`). Plan 30-01 Task 2's own `api[_-]?key\|secret\|token=\|bearer` grep over `commissary.md` re-run: 0 hits. `configuration.md` names `ANTHROPIC_MAX_TOKENS` (a size setting), never a credential variable | closed |
| T-30-02 | Tampering | `docs/src/architecture/commissary.md` usage sketch | medium | mitigate | Plan 30-01 Task 2 anti-invention gate re-run verbatim on 2026-09-14: all 5 page-named methods (`allotted_tokens`, `dispense`, `from_port`, `new`, `verify_fits`) resolve to `pub fn` in `crates/paladin-llm/src/services/commissary.rs`; all 8 page-named types (`Commissary`, `CommissaryError`, `CommissaryPlan`, `Consignment`, `ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile`) resolve to `pub struct`/`pub enum`; exactly 2 `rust,ignore` fences and 1 `mermaid` fence; nav link count 1; `mdbook build docs/` exit 0 | closed |
| T-30-03 | Tampering | Scope creep from documentation into code behaviour | medium | mitigate | `git diff --name-only 8ed14aea..HEAD -- '*.rs' Cargo.toml Cargo.lock MIGRATION.md` over all 29 phase commits lists exactly `crates/paladin-core/src/platform/container/herald.rs` and `src/lib.rs`; non-comment changed lines in each: 0 (`///`-only and `//`-only respectively). `30-03-SUMMARY.md` records `cargo fmt --check` and `cargo check --workspace` exit 0; the UAT commit's pre-commit hook re-ran `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -D warnings` green on 2026-09-14 | closed |
| T-30-04 | Repudiation | ADR-0049 provenance | low | mitigate | `0049-commissary-design-and-rename.md:45,86` cite the historical record by its full branch-qualified path `git show origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`, stated as "never checked out, never treated as a live in-tree ADR"; `git diff --name-status` over `.planning/decisions/` for the phase shows only `A 0049`, `A 0050`, `A 0051`, `M PROMOTION.md` — the in-tree ADR-0010 is untouched | closed |
| T-30-05 | Elevation of privilege | ADR-0051 supersession scope wording | medium | mitigate | `0051-…-x03-supersession.md:30-32`: "X-03 is superseded for Phases 31, 32 and 33 only … extends to no other phase, no other milestone, and no other public API anywhere in v0.10.0"; `## Considered Options` (line 60) records "A blanket X-03 waiver for all of v0.10.0" as rejected (line 65); X-03 cited at its source line (`00-program-overview.md` line 44) | closed |
| T-30-06 | Spoofing | Reserved-term scaffolding for `Treasurer` | medium | mitigate | `find crates src -iname '*treasurer*'` → 0 files/dirs; symbol-scoped grep (`\bTreasurer\b` on non-comment `.rs` lines) → 0; doc-line occurrences → 5, all the `herald.rs` reservation notes plan 30-03 added by design; no `Cargo.toml`/config key/feature flag carries the name (0) | closed |
| T-30-07 | Repudiation | ADR-0050's 0/0 claim vs this phase's own later `herald.rs` edit | medium | mitigate | `0050-treasurer-reservation.md:24-32,84,93` state the authoring-time grep result, the durable *symbol-scoped* 0/0 invariant ("not the weaker 0/0-everywhere claim"), and the `herald.rs` successor state up front, naming plan 30-03 as the source of the first prose mention; commit order confirms 30-02 (ADR-0050) landed before 30-03's `herald.rs` edit | closed |
| T-30-08 | Tampering | `configuration.md` Token Budget Terminology rows | medium | mitigate | Every key in the four rows resolves in the tree: `garrison.max_tokens` → `crates/paladin-memory/src/config/garrison.rs:19` (surfaced via `src/config/settings.rs:41`); `rag.max_tokens` → `crates/paladin-memory/src/config/rag.rs:37` (`settings.rs:43`); `LlmRequest` metadata `"max_tokens"` → `crates/paladin-llm/src/openai/adapter.rs:121,595`, `deepseek/adapter.rs:127,373`; `ANTHROPIC_MAX_TOKENS` → `crates/paladin-llm/src/anthropic/adapter.rs:80`; `agent_runtime.token_budget.max_tokens` → `src/config/agent_runtime.rs:497`. The false `llm.anthropic.max_tokens` YAML key was already removed by review fix WR-01 (`06b25fc8`); `LlmProviderConfig` (`crates/paladin-llm/src/config/llm.rs`) has no such field. See Observations O-30-01..03 for non-threat precision notes | closed |
| T-30-09 | Repudiation | `total_cost()` accessor rustdoc | low | mitigate | `herald.rs:546-550` now reads "Returns the `cost_estimate` field as stored … reserved for the Treasurer (Milestone 14 / FUT-08) … returns `None` in this tree"; `grep -ci fallback herald.rs` → 0, so the former false "calculates a fallback estimate" claim is gone; `cargo test --doc -p paladin-ai-core` passed per `30-03-SUMMARY.md` | closed |
| T-30-10 | Denial of service | History rewrite reaching `.planning/` phase records | medium | mitigate | `git diff --name-status --diff-filter=DR 8ed14aea..HEAD -- .planning/phases` → empty (no deletion or rename of any phase record). The only paths outside `30-…/` are three empty `.gitkeep` placeholders for Phases 31/32/33, added by the planning commit `f22661e3` ("mark Phase 30 planned") as roadmap scaffolding — additions from plan-phase, not from plan 30-03 Task 3's commit, whose own diff touched only this phase's directory as asserted | closed |
| T-30-SC | Tampering | Package-manager installs | low | accept | No dependency added: `git diff --name-only 8ed14aea..HEAD -- Cargo.toml Cargo.lock` is empty across the whole phase; `30-RESEARCH.md` "Package Legitimacy Audit: not applicable". See Accepted Risks R-30-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-30-01 | T-30-SC | The package-legitimacy gate has nothing to audit: the phase adds no dependency and changes neither `Cargo.toml` nor `Cargo.lock` (verified by an empty phase-wide diff on both). Low severity, plan-time disposition in all three plans. | Plans 30-01/02/03 (plan-time disposition) | 2026-09-14 |

*Accepted risks do not resurface in future audit runs.*

---

## Observations (non-threat, non-blocking)

Documentation-precision notes found while closing T-30-08. None names a nonexistent key or induces an ignored setting, so none is a threat; they are recorded for a docs follow-up.

| ID | Location | Note |
|----|----------|------|
| O-30-01 | `configuration.md` Token Budget Terminology, Anthropic cell | Reads "required, env-only"; `AnthropicConfig::from_env()` defaults `ANTHROPIC_MAX_TOKENS` to `4096` when unset (`anthropic/adapter.rs:80-83`, rustdoc line 65 says "optional"). The Messages API parameter is required; the env var is not. |
| O-30-02 | Same table, Garrison and RAG rows, Owner column | Says `src/config/`; the structs live in `crates/paladin-memory/src/config/{garrison,rag}.rs` and are surfaced through `src/config/settings.rs`. Key paths are correct. |
| O-30-03 | Same table, "four independent senses" | `vision.{openai,anthropic}.max_tokens` (`crates/paladin-llm/src/config/vision.rs`, feature `vision`) is a further per-provider `max_tokens` surface not listed. |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-14 | 11 | 11 | 0 | /gsd-secure-phase 30 (orchestrator, L1 short-circuit; anti-invention gate, credential grep and phase-wide diff guard executed locally) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-14
