---
phase: 35
slug: mdbook-currency
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-17
validated: 2026-09-17
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
| 35-01-01 | 01 | 1 | CURR-06, CURR-07, CURR-08 | T-35-01, T-35-02 | No credential or live env value in the new page or module; `docs/` tree unmutated by the mermaid install | build | `grep -cE '^- \[[ x]\] \*\*CURR-(06\|07\|08\|09\|10)\*\*' .planning/REQUIREMENTS.md \| grep -qx 5 && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ✅ green |
| 35-01-02 | 01 | 1 | CURR-06, CURR-08 | T-35-01 | Engine page documents env-var names and defaults, never live values | build | `grep -c '^// ANCHOR: ' crates/doc-examples/src/superstep_engine.rs \| grep -qx 4 && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ✅ green |
| 35-01-03 | 01 | 1 | CURR-06 | — | N/A | file-assertion | `test -f .planning/phases/35-mdbook-currency/deferred-items.md && grep -q 'contributing-providers.md' .planning/phases/35-mdbook-currency/deferred-items.md` | ✅ | ✅ green |
| 35-02-01 | 02 | 2 | CURR-06, CURR-07, CURR-08 | T-35-03, T-35-04 | New module uses `crate::support::` mocks and placeholders only; `lib.rs` diff is additions only | compile + build | `grep -q 'pub mod paladin_agents;' crates/doc-examples/src/lib.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ✅ green |
| 35-02-02 | 02 | 2 | CURR-06, CURR-08 | T-35-03 | No credential in the arsenal or herald anchors | compile + build | `grep -q 'execution_time_ms' crates/doc-examples/src/arsenal_tools.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ✅ green |
| 35-02-03 | 02 | 2 | CURR-06, CURR-08 | T-35-03 | RAG anchors use mock ports, no live endpoint or key | compile + build | `grep -q 'RagRetrievalResult' crates/doc-examples/src/sanctum_vector_memory.rs && ./scripts/check-doc-examples.sh && mdbook build docs/` | ✅ | ✅ green |
| 35-03-01 | 03 | 2 | CURR-06, CURR-07, CURR-08 | T-35-05 | N/A | grep-gate + build | `! grep -qE '"0\.[5-9]\.[0-9]+"' docs/src/getting-started/installation.md && ! grep -qE '\b1\.(70\|75\|85)(\.[0-9]+)?\b' docs/src/getting-started/installation.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-03-02 | 03 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `! grep -rqE '"0\.[5-9]\.[0-9]+"' docs/src/getting-started/quickstart.md docs/src/user-guides/maneuver-flow-dsl.md && mdbook build docs/` | ✅ | ✅ green |
| 35-03-03 | 03 | 2 | CURR-06, CURR-08 | T-35-05 | No planning-corpus reference survives on a shipped page | grep-gate + build | `grep -q 'superstep-engine.md' docs/src/user-guides/control-flow.md && ! grep -q '23-CONTEXT.md' docs/src/user-guides/control-flow.md && mdbook build docs/ && ./scripts/check-doc-examples.sh` | ✅ | ✅ green |
| 35-04-01 | 04 | 2 | CURR-06, CURR-07, CURR-09 | T-35-06 | N/A | grep-gate + build | `grep -q 'superstep-engine.md' docs/src/introduction.md && grep -q 'domain-model.md' docs/src/introduction.md && mdbook build docs/` | ✅ | ✅ green |
| 35-04-02 | 04 | 2 | CURR-06, CURR-08, CURR-09 | T-35-06 | Struct excerpts come from public crate source, no secret | grep-gate + build | `! grep -rqiE '\bQuartermaster\b' docs/src && grep -q 'ConversationRole' docs/src/architecture/domain-model.md && mdbook build docs/` | ✅ | ✅ green |
| 35-04-03 | 04 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'paladin-eval' docs/src/architecture/overview.md && grep -q 'LlmRequest' docs/src/architecture/hexagonal-design.md && mdbook build docs/` | ✅ | ✅ green |
| 35-05-01 | 05 | 2 | CURR-06, CURR-07 | T-35-07, T-35-08 | Every ADR link is a blob URL under the single canonical repository host | link-check | `grep -c 'blob/main/.planning/decisions/' docs/src/contributing/adr-index.md \| grep -qE '^(9\|1[0-9])$' && mdbook build docs/` | ✅ | ✅ green |
| 35-05-02 | 05 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'mem --> llm' docs/src/api-reference/crate-map.md && grep -q 'rust:1.93-slim-bookworm' docs/src/api-reference/feature-flags.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-05-03 | 05 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'paladin_core::platform::container' docs/src/api-reference/stable-api.md && git diff --quiet HEAD -- docs/src/api-reference/upgrading.md && mdbook build docs/` | ✅ | ✅ green |
| 35-06-01 | 06 | 2 | CURR-06, CURR-07, CURR-08 | T-35-09, T-35-10 | CodeQL described as advisory-only and non-merge-gating; YAML excerpts carry no secret name | grep-gate + yaml-parse | `grep -q 'codeql.yml' docs/src/deployment/cicd.md && ! grep -q 'build-release' docs/src/deployment/cicd.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-06-02 | 06 | 2 | CURR-06, CURR-08 | T-35-09 | The documented coverage command is the command CI runs | grep-gate + yaml-parse | `grep -q 'integration-tests,llm-all' docs/src/contributing/testing-guide.md && ! grep -q 'workflows/test.yml' docs/src/contributing/testing-guide.md && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-06-03 | 06 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q 'engine_benchmarks.rs' docs/src/operations/performance-tuning.md && ! grep -rq 'Corrected 2026-09' docs/src/operations && mdbook build docs/` | ✅ | ✅ green |
| 35-07-01 | 07 | 2 | CURR-06, CURR-07, CURR-08 | T-35-11, T-35-12 | Captured `--help` carries no key, token or absolute local path; fabricated flags deleted not annotated | cli-capture + build | `grep -q 'paladin-cli council --help' docs/src/appendix/cli-council.md && ! grep -q -- '--synthesize' docs/src/appendix/cli-council.md && mdbook build docs/` | ✅ | ✅ green |
| 35-07-02 | 07 | 2 | CURR-06, CURR-08 | T-35-11, T-35-12 | No fabricated environment variable survives | cli-capture + build | `! grep -qE 'PALADIN_ENV_FILE\|PALADIN_SKIP_VALIDATION' docs/src/appendix/cli-onboarding.md && ! grep -q -- '--pattern' docs/src/appendix/cli-muster.md && mdbook build docs/` | ✅ | ✅ green |
| 35-07-03 | 07 | 2 | CURR-06, CURR-08 | T-35-12 | Every short alias shown exists on the live argument attribute | count-check + build | `T4=$(grep -cE '#\[test\]\|#\[tokio::test\]' tests/integration/llm_live_api_tests.rs) && grep -q "$T4" docs/src/appendix/cli-testing.md && grep -q -- '--features cli' docs/src/appendix/cli-usage.md` | ✅ | ✅ green |
| 35-08-01 | 08 | 2 | CURR-06, CURR-07 | T-35-13 | An archived snapshot can no longer read as current assurance | grep-gate + build | `head -6 docs/src/appendix/doc-coverage-report.md \| grep -q '^> \*\*Archived' && grep -q 'ADR-0033' docs/src/appendix/doc-coverage-report.md && mdbook build docs/` | ✅ | ✅ green |
| 35-08-02 | 08 | 2 | CURR-06, CURR-07 | T-35-13 | Each banner names the live source; nav entries preserved | fence-balance + build | `awk '/^```/{n++} END{exit n%2}' docs/src/appendix/user-rest-api.md && grep -q '1.88' docs/src/appendix/contributing-legacy.md && mdbook build docs/` | ✅ | ✅ green |
| 35-08-03 | 08 | 2 | CURR-06, CURR-08 | — | N/A | grep-gate + build | `grep -q '1.88' docs/src/appendix/battalion-benchmarks.md && grep -q 'check-release-consistency' docs/src/appendix/release-automation.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-09-01 | 09 | 2 | CURR-06, CURR-07, CURR-08 | T-35-15 | The throwaway compile probe never enters the tree | scratch-compile + build | `! grep -q 'paladin::paladin_ports::' docs/src/appendix/minio-file-repository-setup.md && test -z "$(git status --porcelain -- examples)" && mdbook build docs/` | ✅ | ✅ green |
| 35-09-02 | 09 | 2 | CURR-06, CURR-08 | T-35-15 | Probe file deleted after each use | scratch-compile + grep-gate | `! grep -rq 'paladin::paladin_ports::' docs/src && test "$(grep -rc 'paladin::infrastructure::adapters::llm::' docs/src \| grep -v ':0$')" = "docs/src/contributing/contributing-providers.md:2" && ! grep -qE 'paladin::infrastructure::adapters::llm::\|OpenAiAdapter' docs/src/appendix/{minio-file-repository-setup,redis-queue-adapter-setup,sanctum-migration,port-trait-template,provider-expansion,sentinel}.md && test -z "$(git status --porcelain -- examples)"` | ✅ | ✅ green |
| 35-09-03 | 09 | 2 | CURR-06, CURR-08 | T-35-14 | Snyk removal and CodeQL advisory-only disposition stated as measured, never as assurance | grep-gate + build | `grep -qi 'evaluated and removed' docs/src/appendix/security-scanning.md && grep -qi 'CodeQL' docs/src/appendix/security-scanning.md && mdbook build docs/ && ./scripts/check-doc-config.sh` | ✅ | ✅ green |
| 35-10-01 | 10 | 3 | CURR-06, CURR-07, CURR-08, CURR-09 | T-35-16, T-35-17 | `make api-surface` proves no public-surface change; allowlist rows carry reasons | full-gate | `mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh && make api-surface` | ✅ | ✅ green |
| 35-10-02 | 10 | 3 | CURR-10 | — | No planning identifier leaks into the release record | grep-gate | `awk '/^## \[0\.10\.0\]/{i=1} /^## \[0\.9/{i=0} i && /^### Documentation/{d=1} END{exit !d}' CHANGELOG.md && ! grep -qE '\bMB-[0-9]{2}\b' CHANGELOG.md` | ✅ | ✅ green |
| 35-10-03 | 10 | 3 | CURR-06, CURR-07, CURR-09 | T-35-16, T-35-17 | Final recorded run is the one Phase 37 re-seals against | full-gate | `mdbook-mermaid install docs/ && test -z "$(git status --porcelain -- docs)" && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh && make api-surface` | ✅ | ✅ green |

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
| Book-wide `OpenAiAdapter` casing sweep is empty (the live struct is `OpenAIAdapter`) | CURR-08 (ROADMAP SC3) — residual, deferred | Not a D-21 exit grep and not an `MB-nn` row; 7 pages still carry the casing (`grep -rlE '\bOpenAiAdapter\b' docs/src`), recorded as a deferred observation by plan 35-09 (D-27) with owner unassigned. Fixing them is a docs edit outside the minted work list, so this audit cannot generate a test that goes green without changing pages the phase chose not to touch | Before the next docs-currency pass: `grep -rnE '\bOpenAiAdapter\b' docs/src` must be empty; then fold the sweep back into row 35-09-02's automated command |
| `contributing/contributing-providers.md` lines 272 and 367 still import `paladin::infrastructure::adapters::llm::…` | CURR-08 (ROADMAP SC3) — residual, allowlisted | Allowlisted in `35-EVIDENCE.md` §4 under D-21 (page is not in the 60-row work list; deferred by plan 35-01, D-27). Row 35-09-02's automated command pins the residual to exactly these two lines so any new occurrence turns the row red | `grep -rn 'paladin::infrastructure::adapters::llm::' docs/src` returns only the two allowlisted lines until a future pass applies the D-13 fix to that page |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 90s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-09-17 by `/gsd-validate-phase 35` (every row re-run on HEAD `632ff2f6`, see audit below)

---

## Validation Audit 2026-09-17

| Metric | Count |
|--------|-------|
| Rows audited | 30 |
| Gaps found | 3 |
| Resolved | 3 (all command corrections) |
| Escalated | 0 (one residual moved to Manual-Only, see below) |

All 30 per-task rows were executed verbatim on HEAD `632ff2f6` (not read from the SUMMARYs) with
the seeded commands; 27 were green first time and 3 failed before any build ran. None of the three
was a missing test, so no `gsd-nyquist-auditor` spawn was needed — the same disposition as the
Phase 25 and Phase 33 audits. Per-row wall clock ranged 0–10 s on a warm target (`mdbook build`
≈2–5 s, the two full-gate rows 35-10-01/35-10-03 ≈9–10 s including `make api-surface`), well inside
the 90 s latency budget.

**Corrections applied to the map:**

1. **35-01-01 — assertion drifted with phase completion.** The seeded grep required the five
   CURR rows to be *unchecked* (`- [ ] **CURR-0N**`). `phase.complete 35` flipped them to `[x]`
   (`.planning/REQUIREMENTS.md:495-512`), so the row read red on a tree where the intended
   fact — five minted rows exist — holds. Command now accepts either checkbox state.
2. **35-07-03 — markdown-escape ambiguity.** The seeded `grep -c '#\[test\]\|#\[tokio::test\]'`
   relied on GNU BRE `\|` alternation, which is indistinguishable in a markdown table cell from an
   escaped shell pipe; unescaped to `|` it counts 0 and `grep -c` exits 1. Switched to `grep -cE`
   so the alternation survives either reading. Live count is 13, matching `cli-testing.md:91`.
3. **35-09-02 — plan verify was stricter than the phase's own exit criteria.** Plan 35-09 Task 2's
   `<automated>` block asserted three book-wide empties: `paladin::paladin_ports::`,
   `paladin::infrastructure::adapters::llm::` and `OpenAiAdapter`. On HEAD the first is empty; the
   second has exactly the two `contributing-providers.md` lines that `35-EVIDENCE.md` §4
   allowlists under D-21; the third has 7 hits. CONTEXT.md D-21 names only the first two as D-13
   exit greps and permits allowlisted residue, and the executor recorded the casing residual
   transparently (`35-09-SUMMARY.md` §Deferred observations, folded into `deferred-items.md`).
   The row's command is now the D-21 form — `paladin_ports` empty book-wide, the adapter-path
   grep pinned to exactly the two allowlisted lines, and all three patterns absent from the six
   D-13 appendix pages the plan owned — which is green. The book-wide casing sweep moved to
   Manual-Only with the deferred pointer.

**Flagged for the milestone audit, not a Nyquist gap.** CURR-08's literal wording is "no touched
page names a type … the v0.10.0 tree does not export". Four pages that *were* in a Phase 35 plan's
`files_modified` still name `OpenAiAdapter` on lines outside the `MB-nn` row each plan closed:
`appendix/battalion-patterns-guide.md` (35-09, body of the four examples — scoped out explicitly),
`contributing/testing-guide.md:324` (35-06), `contributing/architecture-decisions.md:236` (35-05),
`user-guides/tool-integration.md:187,295` (35-03); three more pages outside the phase carry it too.
`35-VERIFICATION.md` passed CURR-08 on the compile gate and the D-21 greps, and `deferred-items.md`
registers the casing defect with owner unassigned. One factual slip in that register: it describes
the pages as "entirely outside this phase's `files_modified` lists", which is true for only three
of the six it names. The register is folded and closed, so it is corrected here rather than edited.
A single `sed -i 's/\bOpenAiAdapter\b/OpenAIAdapter/g'` quick task over the 7 pages would let the
Manual-Only row fold back into 35-09-02.
