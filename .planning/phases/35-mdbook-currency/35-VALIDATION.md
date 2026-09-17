---
phase: 35
slug: mdbook-currency
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-17
---

# Phase 35 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdBook build + linkcheck backend, `scripts/check-doc-examples.sh` (Layer 1 `cargo check` on `crates/doc-examples`, Layer 2 inline-block scan), `scripts/check-doc-config.sh` (YAML fence parse) — no unit-test framework applies to prose pages |
| **Config file** | `docs/book.toml` (`[output.linkcheck] warning-policy = "error"`, `follow-web-links = false`) |
| **Quick run command** | `mdbook build docs/` |
| **Full suite command** | `mdbook-mermaid install docs/ && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh` (the exact `docs.yml` sequence) |
| **Estimated runtime** | ~4 seconds quick (measured 3.3 s); ~90 seconds full when `doc-examples` needs a warm `cargo check` |

---

## Sampling Rate

- **After every task commit:** Run `mdbook build docs/` (plus `./scripts/check-doc-examples.sh` after any `crates/doc-examples` edit)
- **After every plan wave:** Run `mdbook-mermaid install docs/ && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh`
- **Before `/gsd-verify-work`:** Full suite must be green, plus the CONTEXT.md D-21 exit greps and `make api-surface`, recorded verbatim in `35-EVIDENCE.md`
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 35-01-01 | 01 | 1 | CURR-06, CURR-07, CURR-08 | T-35-01, T-35-02 | No credential or live env value in the new page or module; `docs/` tree unmutated by the mermaid install | build | `grep -cE '^- \[ \] \*\*CURR-(06\|07\|08\|09\|10)\*\*' .planning/REQUIREMENTS.md \| grep -qx 5 && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ⬜ pending |
| 35-01-02 | 01 | 1 | CURR-06, CURR-08 | T-35-01 | Engine page documents env-var names and defaults, never live values | build | `grep -c '^// ANCHOR: ' crates/doc-examples/src/superstep_engine.rs \| grep -qx 4 && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ⬜ pending |
| 35-01-03 | 01 | 1 | CURR-06 | — | N/A | file-assertion | `test -f .planning/phases/35-mdbook-currency/deferred-items.md && grep -q 'contributing-providers.md' .planning/phases/35-mdbook-currency/deferred-items.md` | ✅ | ⬜ pending |
| 35-02-01 | 02 | 2 | CURR-06, CURR-07, CURR-08 | T-35-03, T-35-04 | New module uses `crate::support::` mocks and placeholders only; `lib.rs` diff is additions only | compile + build | `grep -q 'pub mod paladin_agents;' crates/doc-examples/src/lib.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ⬜ pending |
| 35-02-02 | 02 | 2 | CURR-06, CURR-08 | T-35-03 | No credential in the arsenal or herald anchors | compile + build | `grep -q 'execution_time_ms' crates/doc-examples/src/arsenal_tools.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ⬜ pending |
| 35-02-03 | 02 | 2 | CURR-06, CURR-08 | T-35-03 | RAG anchors use mock ports, no live endpoint or key | compile + build | `grep -q 'RagRetrievalResult' crates/doc-examples/src/sanctum_vector_memory.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ⬜ pending |
| 35-03-01 | 03 | 2 | CURR-06, CURR-07, CURR-08 | T-35-05 | N/A | grep-gate + build | `! grep -qE '"0\.[5-9]\.[0-9]+"' docs/src/getting-started/installation.md && ! grep -qE '\b1\.(70\|75\|85)(\.[0-9]+)?\b' docs/src/getting-started/installation.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-03-02 | 03 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `! grep -rqE '"0\.[5-9]\.[0-9]+"' docs/src/getting-started/quickstart.md docs/src/user-guides/maneuver-flow-dsl.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-03-03 | 03 | 2 | CURR-06, CURR-08 | T-35-05 | No planning-corpus reference survives on a shipped page | grep-gate + build | `grep -q 'superstep-engine.md' docs/src/user-guides/control-flow.md && ! grep -q '23-CONTEXT.md' docs/src/user-guides/control-flow.md && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ⬜ pending |
| 35-04-01 | 04 | 2 | CURR-06, CURR-07, CURR-09 | T-35-06 | N/A | grep-gate + build | `grep -q 'superstep-engine.md' docs/src/introduction.md && grep -q 'domain-model.md' docs/src/introduction.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-04-02 | 04 | 2 | CURR-06, CURR-08, CURR-09 | T-35-06 | Struct excerpts come from public crate source, no secret | grep-gate + build | `! grep -rqiE '\bQuartermaster\b' docs/src && grep -q 'ConversationRole' docs/src/architecture/domain-model.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-04-03 | 04 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'paladin-eval' docs/src/architecture/overview.md && grep -q 'LlmRequest' docs/src/architecture/hexagonal-design.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-05-01 | 05 | 2 | CURR-06, CURR-07 | T-35-07, T-35-08 | Every ADR link is a blob URL under the single canonical repository host | link-check | `grep -c 'blob/main/.planning/decisions/' docs/src/contributing/adr-index.md \| grep -qE '^(9\|1[0-9])$' && mdbook build docs/` | ✅ | ⬜ pending |
| 35-05-02 | 05 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'mem --> llm' docs/src/api-reference/crate-map.md && grep -q 'rust:1.93-slim-bookworm' docs/src/api-reference/feature-flags.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-05-03 | 05 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'paladin_core::platform::container' docs/src/api-reference/stable-api.md && git diff --quiet HEAD -- docs/src/api-reference/upgrading.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-06-01 | 06 | 2 | CURR-06, CURR-07, CURR-08 | T-35-09, T-35-10 | CodeQL described as advisory-only and non-merge-gating; YAML excerpts carry no secret name | grep-gate + yaml-parse | `grep -q 'codeql.yml' docs/src/deployment/cicd.md && ! grep -q 'build-release' docs/src/deployment/cicd.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-06-02 | 06 | 2 | CURR-06, CURR-08 | T-35-09 | The documented coverage command is the command CI runs | grep-gate + yaml-parse | `grep -q 'integration-tests,llm-all' docs/src/contributing/testing-guide.md && ! grep -q 'workflows/test.yml' docs/src/contributing/testing-guide.md && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-06-03 | 06 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'engine_benchmarks.rs' docs/src/operations/performance-tuning.md && ! grep -rq 'Corrected 2026-09' docs/src/operations && mdbook build docs/` | ✅ | ⬜ pending |
| 35-07-01 | 07 | 2 | CURR-06, CURR-07, CURR-08 | T-35-11, T-35-12 | Captured `--help` carries no key, token or absolute local path; fabricated flags deleted not annotated | cli-capture + build | `grep -q 'paladin-cli council --help' docs/src/appendix/cli-council.md && ! grep -q -- '--synthesize' docs/src/appendix/cli-council.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-07-02 | 07 | 2 | CURR-06, CURR-08 | T-35-11, T-35-12 | No fabricated environment variable survives | cli-capture + build | `! grep -qE 'PALADIN_ENV_FILE\|PALADIN_SKIP_VALIDATION' docs/src/appendix/cli-onboarding.md && ! grep -q -- '--pattern' docs/src/appendix/cli-muster.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-07-03 | 07 | 2 | CURR-06, CURR-08 | T-35-12 | Every short alias shown exists on the live argument attribute | count-check + build | `T4=$(grep -c '#\[test\]\|#\[tokio::test\]' tests/integration/llm_live_api_tests.rs) && grep -q "$T4" docs/src/appendix/cli-testing.md && grep -q -- '--features cli' docs/src/appendix/cli-usage.md` | ✅ | ⬜ pending |
| 35-08-01 | 08 | 2 | CURR-06, CURR-07 | T-35-13 | An archived snapshot can no longer read as current assurance | grep-gate + build | `head -6 docs/src/appendix/doc-coverage-report.md \| grep -q '^> \*\*Archived' && grep -q 'ADR-0033' docs/src/appendix/doc-coverage-report.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-08-02 | 08 | 2 | CURR-06, CURR-07 | T-35-13 | Each banner names the live source; nav entries preserved | fence-balance + build | `awk '/^```/{n++} END{exit n%2}' docs/src/appendix/user-rest-api.md && grep -q '1.88' docs/src/appendix/contributing-legacy.md && mdbook build docs/` | ✅ | ⬜ pending |
| 35-08-03 | 08 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q '1.88' docs/src/appendix/battalion-benchmarks.md && grep -q 'check-release-consistency' docs/src/appendix/release-automation.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-09-01 | 09 | 2 | CURR-06, CURR-07, CURR-08 | T-35-15 | The throwaway compile probe never enters the tree | scratch-compile + build | `! grep -q 'paladin::paladin_ports::' docs/src/appendix/minio-file-repository-setup.md && test -z "$(git status --porcelain -- examples)" && mdbook build docs/` | ✅ | ⬜ pending |
| 35-09-02 | 09 | 2 | CURR-06, CURR-08 | T-35-15 | Probe file deleted after each use | scratch-compile + grep-gate | `! grep -rq 'paladin::paladin_ports::' docs/src && ! grep -rq 'paladin::infrastructure::adapters::llm::' docs/src && ! grep -rq 'OpenAiAdapter' docs/src && test -z "$(git status --porcelain -- examples)"` | ✅ | ⬜ pending |
| 35-09-03 | 09 | 2 | CURR-06, CURR-08 | T-35-14 | Snyk removal and CodeQL advisory-only disposition stated as measured, never as assurance | grep-gate + build | `grep -qi 'evaluated and removed' docs/src/appendix/security-scanning.md && grep -qi 'CodeQL' docs/src/appendix/security-scanning.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ⬜ pending |
| 35-10-01 | 10 | 3 | CURR-06, CURR-07, CURR-08, CURR-09 | T-35-16, T-35-17 | `make api-surface` proves no public-surface change; allowlist rows carry reasons | full-gate | `mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh && make api-surface` | ✅ | ⬜ pending |
| 35-10-02 | 10 | 3 | CURR-10 | — | No planning identifier leaks into the release record | grep-gate | `awk '/^## \[0\.10\.0\]/{i=1} /^## \[0\.9/{i=0} i && /^### Documentation/{d=1} END{exit !d}' CHANGELOG.md && ! grep -qE '\bMB-[0-9]{2}\b' CHANGELOG.md` | ✅ | ⬜ pending |
| 35-10-03 | 10 | 3 | CURR-06, CURR-07, CURR-09 | T-35-16, T-35-17 | Final recorded run is the one Phase 37 re-seals against | full-gate | `mdbook-mermaid install docs/ && test -z "$(git status --porcelain -- docs)" && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh && make api-surface` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*(Seeded by plan-phase from `35-RESEARCH.md` §Validation Architecture; the planner fills one row per task and `/gsd-validate-phase 35` sets `nyquist_compliant`.)*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements: the phase's tests ARE the `docs.yml` gate sequence (already wired into CI as the required "Build MDBook" check), the `crates/doc-examples` compile gate, and the exit greps in CONTEXT.md D-21 — no new test file, fixture or framework is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `CHANGELOG.md` `[0.10.0]` `### Documentation` subsection summarises the pages added and corrected | CURR-10 (ROADMAP SC5) | Content adequacy is a reading judgment; presence is automated | `grep -n -A 8 '^### Documentation' CHANGELOG.md` after the final plan, then read the bullets against the closure table |
| Each corrected page's prose describes the shipped surface accurately | CURR-08 (ROADMAP SC3) | Type/flag names are grep-checkable; prose meaning is not | Re-run `.planning/phases/34-documentation-currency-audit/34-signals.sh <page>` and read the row's §2 finding against the edited page |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
