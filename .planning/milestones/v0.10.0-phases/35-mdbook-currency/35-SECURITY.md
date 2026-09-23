---
phase: 35
slug: mdbook-currency
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-17
---

# Phase 35 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 35 is a documentation-currency phase: it edits `docs/src` prose, adds six compile-checked
example modules under `crates/doc-examples`, and touches no runtime code path, network endpoint,
auth path or schema. The threat register below is the union of the `<threat_model>` blocks in
plans 35-01 through 35-10 (register authored at plan time). Every mitigation was verified against
the committed tree on 2026-09-17 by direct inspection; ASVS L1 grep-depth applies.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| repository → published book (`docs/book/html/` via GitHub Pages) | Anything written into `docs/src` is published | Prose, code excerpts, outbound links — public |
| planning corpus → shipped page | `.planning/` content quoted into a page becomes public | ADR text, phase-context references — internal until quoted |
| workspace → `crates/doc-examples` | Six new compile-checked Rust modules enter the workspace build graph | First-party source only; mock adapters |
| published book → github.com | The ADR index emits outbound links the reader follows | URLs — must stay on the canonical repository host |
| CI configuration / security posture record → documentation | The book states which gates protect `main`; a wrong statement reads as assurance the project does not have | Ruleset contents, scanner dispositions |
| local shell → documentation | `--help` output from a locally built binary is pasted verbatim into a published page | Captured stdout — may echo environment-derived defaults |
| local shell → repository | The D-11(c) throwaway compile probe writes into `examples/` | Scratch Rust — must never be staged |
| historical snapshot → current reader | An unbannered archived page reads as current guidance | Superseded claims (coverage, CLI shape) |
| repository → crates.io | `make api-surface` proves this phase moved no public surface before Phase 37 publishes | Public API baseline |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-35-01 | Information Disclosure | `docs/src/user-guides/superstep-engine.md`, `crates/doc-examples/src/superstep_engine.rs` | low | mitigate | Module builds from `crate::support::` mocks; credential-pattern grep over module and page returns nothing; the `APP_ENGINE_*` table (page lines 98–104) names variables and bounds only, no live values | closed |
| T-35-02 | Tampering | `docs/` working tree after `mdbook-mermaid install docs/` | low | mitigate | `git status --porcelain -- docs` is empty; `git ls-files docs` tracks no `mermaid*` asset; no phase commit touches `docs/theme` or `docs/book.toml` | closed |
| T-35-03 | Information Disclosure | five new `crates/doc-examples` modules and their five pages | low | mitigate | Modules depend on `MockLlmAdapter`, `MockEmbedder` and `crate::support::` only; the single credential-shaped grep hit is `BRAVE_API_KEY: "${BRAVE_API_KEY}"` in `arsenal-tools.md:54` — an env-interpolation placeholder, not a value | closed |
| T-35-04 | Tampering | `crates/doc-examples/src/lib.rs` | low | mitigate | `git log -p` over the phase commits shows exactly six `+pub mod …;` lines on `lib.rs` (one per MB row) and no removals or edits | closed |
| T-35-05 | Information Disclosure | `docs/src/user-guides/control-flow.md` and the eight sibling pages | low | mitigate | `grep -c '23-CONTEXT.md' control-flow.md` → 0; no `-CONTEXT.md` reference remains anywhere under `docs/src/user-guides/` | closed |
| T-35-06 | Information Disclosure | `docs/src/architecture/domain-model.md`, `docs/src/architecture/commissary.md` | low | mitigate | ADR-0049 is cited by ID and file path (`commissary.md:7-8`, `domain-model.md:34`); struct excerpts are from public `paladin-core` source; no deliberation text is quoted | closed |
| T-35-07 | Spoofing | `docs/src/contributing/adr-index.md` outbound links | medium | mitigate | All 9 Record links share the prefix `https://github.com/DF3NDR/paladin-dev-env/blob/main/.planning/decisions/`, matching `Cargo.toml` `repository`; zero links to any other host | closed |
| T-35-08 | Information Disclosure | the ADR index's link targets | low | mitigate | The index is a 16-line link table; it copies no ADR body text | closed |
| T-35-09 | Repudiation | `docs/src/deployment/cicd.md`, `docs/src/contributing/testing-guide.md` | medium | mitigate | Every `Required` row in the `cicd.md` job table (lines 56–84) maps to a `required_status_checks` context in `.github/rulesets/protect-main-branch.json`; every `Advisory` row (MSRV, Semver, Docker Build, Kubernetes Smoke, Publish Dry Run, live-server suites) is absent from the ruleset; `codeql.yml` is captioned "advisory only, does not gate a merge" on both pages (`cicd.md:35`, `testing-guide.md:649`) | closed |
| T-35-10 | Information Disclosure | retained YAML excerpts on the two CI pages | low | mitigate | No token value or runner credential appears; the only `secrets.*` tokens are GitHub expression references (`${{ secrets.KUBE_CONFIG }}`, `OPENAI_API_KEY`, `SLACK_WEBHOOK`) inside the pre-existing illustrative "Deploy to Kubernetes" / "Best Practices" blocks (`cicd.md:280–430`, blame 2026-01-27, unchanged by this phase), and none of those names corresponds to a workflow under `.github/`. Residual is a currency defect (D-15 wanted only captioned excerpts of real jobs), not a disclosure — logged in `deferred-items.md` | closed |
| T-35-11 | Information Disclosure | the five captured `--help` blocks | medium | mitigate | The five `Usage: paladin-cli …` fences (`cli-council`, `cli-muster`, `cli-onboarding`, `cli-setup-check`, `cli-usage`) contain no absolute path, URL host, `sk-` token or `/home`/`/Users`/`/tmp` string; 35-07-SUMMARY records the pre-paste read. `sk-...` and `/home/user/workspace` on `cli-onboarding.md`/`cli-configuration.md` are documented placeholders outside the help fences | closed |
| T-35-12 | Spoofing | fabricated flags and environment variables on the CLI pages | medium | mitigate | All three negative `! grep -q` acceptance asserts from 35-07-PLAN re-run clean against the current tree — no fabricated flag or variable name is present | closed |
| T-35-13 | Repudiation | the five archive-tier pages | medium | mitigate | `doc-coverage-report`, `build-baselines`, `user-system`, `user-rest-api`, `contributing-legacy` each open with `> **Archived — historical document.**` and each keeps its `docs/src/SUMMARY.md` entry | closed |
| T-35-14 | Repudiation | `docs/src/appendix/security-scanning.md` | high | mitigate | Page records Snyk as "evaluated and removed (2026-08-18)" with the measured zero-Rust-coverage result (lines 104–119), CodeQL as "disqualified … retained, advisory-only" (lines 136–156), and lists exactly the five advisories in `.cargo/audit.toml`'s `ignore` array (`2023-0071`, `2025-0111`, `2026-0187`, `2026-0194`, `2026-0195`) | closed |
| T-35-15 | Tampering | `examples/_scratch.rs` | low | mitigate | File absent from the tree, absent from `git log --all`, and `git status --porcelain -- examples` is empty | closed |
| T-35-16 | Tampering | the public API surface baseline | medium | mitigate | `make api-surface` on the current tree: "API surface unchanged" (3959 items); no phase commit touches `.project/` | closed |
| T-35-17 | Repudiation | `35-EVIDENCE.md` allowlist table | medium | mitigate | All 10 allowlist rows carry `File:line`, matched text and a real-file / out-of-scope reason; the evidence file states no row lacks a reason | closed |
| T-35-SC | Tampering | npm/pip/cargo installs | low | accept | See Accepted Risks Log AR-35-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-35-01 | T-35-SC | No registry package was installed in any of the ten plans. The only dependency change is `paladin-memory = { path = "../paladin-memory" }` added to `crates/doc-examples/Cargo.toml` (a first-party workspace path dependency; `Cargo.lock` gains the one corresponding entry). The three mdBook tools are pre-existing `docs.yml` pins reinstalled with `--locked` at fixed versions and recorded in `35-EVIDENCE.md`. `35-RESEARCH.md` records the Package Legitimacy Audit as not applicable. | plan authors (35-01…35-10), confirmed at audit | 2026-09-17 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-17 | 18 | 18 | 0 | /gsd-secure-phase 35 (orchestrator, L1 short-circuit — register authored at plan time, ASVS L1, no open threats) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-17
