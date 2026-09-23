---
phase: 33
slug: commissary-in-tree-adoption
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-17
---

# Phase 33 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — all six PLAN files (33-01 through 33-06) carry a
`<threat_model>` block. Verification depth: ASVS L1 (grep-level presence of each mitigation in the
tree at HEAD `4f840f3f`), per `workflow.security_asvs_level: 1`. Block threshold:
`workflow.security_block_on: high`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| stored memory content → log sink / error text | Retrieved memory bodies are user/agent-authored text; they cross into labels, log lines and error Display in `RagRetrievalService::ration` | Memory bodies (sensitive, user-authored); only UUID labels and counts may cross |
| retained/shed memory data → rendered system prompt | The omission marker and the rendered bodies cross into the LLM-visible prompt in both renderers (`format_for_prompt` and the facade's `format_retrieved_rag_context`) | Memory bodies (already LLM-visible by design) plus the marker (count + budget) |
| config value → allocator budget | `rag.max_tokens` (`usize`, operator-supplied) crosses into the Commissary's `u32` window | Operator integer; over-range must fail typed, never clamp |
| retrieval result → operator log sink | The RAG-success `info!` line in the facade and the `RAG rationing` `info!` line in `paladin-memory` cross into logs an operator reads | Counts and token numbers only |
| workspace crate graph → published dependency graph | `paladin-memory` gains its first unconditional production edge to `paladin-llm` | Transitive dependency set of every downstream `paladin-memory` consumer |
| stored memory content → test assertions and CI logs | `tests/integration/rag_commissary_test.rs` stores and renders memory-shaped text | Synthetic fixture strings only |
| documentation claims → operator expectation | `docs/src/getting-started/configuration.md` and `docs/src/architecture/commissary.md` are what an operator reads to size `rag.max_tokens` | Behavioural claim (planned at pessimistic ratio, not exact) |
| measured tool output → release register | What `cargo semver-checks` printed is the only legitimate source for a `.cargo/semver-checks-allowlist.toml` entry | Allowlist entries and MIGRATION.md §9.2 rows |
| release register → downstream consumer | MIGRATION.md §9.2 and CHANGELOG `[0.10.0]` are what a downstream maintainer reads before upgrading | Release notes |
| measured gate output → release evidence | `.project/v0.10.0/09-program-acceptance-audit.md` §11 is what a maintainer reads before cutting the `v0.10.0` tag | Gate verdicts, head SHA, date |
| agent authority → human sign-off | The sign-off boxes in the acceptance audit are the explicit human-only control; the agent writes evidence beneath them and never closes them | Checkbox state |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-33-01 | Information Disclosure | `ration` → `ConsignmentItem.label` / `ShedItem.label` / `log::info!`; integration-test shed-label assertions (plans 33-01, 33-04) | high | mitigate | `rag_retrieval_service.rs:286` sets `label = result.entry.memory.id.to_string()`; the `RAG rationing` `info!` at `:373-379` interpolates only `retained`, `shed`, `prompt_tokens`, `allotted_tokens`, `exact_tally`. Unit test at `:812` and `tests/integration/rag_commissary_test.rs:149` assert every shed label parses via `Uuid::parse_str`; integration fixtures are synthetic repeated-character strings | closed |
| T-33-02 | Denial of Service | `u32::try_from(self.config.max_tokens)` budget conversion (plans 33-01, 33-03) | medium | mitigate | `rag_retrieval_service.rs:266` uses `u32::try_from(...).map_err(...)` into the typed `RagRetrievalError::BudgetTooLarge { max_tokens }`; negative grep for `as u32` / `.min(u32` / `saturating` / `clamp` finds only doc comments. Tests `budget_beyond_u32_returns_typed_error_and_never_clamps` (`:1080`) and the `:1170` regression assert the typed error and no reduced-budget `Ok` | closed |
| T-33-03 | Tampering (supply chain) | `crates/paladin-memory/Cargo.toml` new `paladin-llm` edge | high | mitigate | `Cargo.toml:34`: `paladin-llm = { version = "0.10.0", path = "../paladin-llm", default-features = false }` with no feature list; `cargo tree -p paladin-memory -i reqwest` reports `did not match any packages` (reqwest absent from the crate's graph) | closed |
| T-33-04 | Information Disclosure | `RagRetrievalError` Display reaching `PaladinError::ExecutionError` and the facade `warn!` | medium | mitigate | `RagRetrievalError` variants carry only: a wrapped `SanctumError`/`CommissaryError`, the configured `max_tokens` number, or a UUID `label` (`UnmatchedDispensedLabel`, and the review-fix-added `DuplicateMemoryId`). No variant holds a memory body | closed |
| T-33-05 | Information Disclosure | facade `info!` at the RAG call site | high | mitigate | `paladin_execution_service.rs:1320-1326` interpolates `execution_id`, `memories`, `shed=results.shed.len()`, `latency_ms` only; no body/content field appears in the statement | closed |
| T-33-06 | Tampering (silent divergence) | two renderers emitting the same user-visible marker | medium | mitigate | Single `pub fn rag_omission_marker` at `rag_retrieval_service.rs:153`; re-exported through `src/application/services/sanctum/mod.rs:9,23` and consumed by the facade renderer at `paladin_execution_service.rs:1995`; byte-identity test `test_format_retrieved_rag_context_ends_with_shared_omission_marker` (`:4541`) asserts `ends_with(&rag_omission_marker(1, 1_234))` | closed |
| T-33-07 | Repudiation | a prompt that was silently reduced | high | mitigate | Marker emitted when `shed` non-empty in both renderers; asserted in both directions: `format_for_prompt_ends_with_marker_when_shed_nonempty` (`:869`), `format_for_prompt_contains_no_marker_when_shed_empty` (`:891`), `:907`, `:923`; facade `test_format_retrieved_rag_context_no_marker_when_shed_empty` (`:4568`); integration `large_budget_sheds_nothing_and_emits_no_marker` | closed |
| T-33-08 | Information Disclosure | proptest failure output | low | accept | Generated bodies are random strings (`proptest!` block at `:940`), not real memory content; a shrink report prints generated data only. See Accepted Risks Log AR-33-02 | closed |
| T-33-09 | Tampering | a future change quietly reordering equal-score memories | low | mitigate | Test `equal_scores_keep_insertion_order_and_distinct_priorities` (`:1197`) pins the stable-sort contract; `ration_respects_budget_and_rank_order` proptest (`:964`) covers rank ordering | closed |
| T-33-10 | Repudiation | a grep-only proof standing in for semantic absence of other truncation sites | medium | accept | Recorded as a flagged assumption in 33-04-PLAN and 33-04-SUMMARY: the sweep is an identifier search; the disposition of every other truncation site is a reviewed judgement, not a machine check. See AR-33-03 | closed |
| T-33-11 | Tampering | a doc page that overstates the enforcement | low | mitigate | `docs/src/architecture/commissary.md:166` states the cap is planned at the Commissary's pessimistic `pessimistic_tokens_per_1000_bytes` ratio; `docs/src/getting-started/configuration.md:416` `rag.max_tokens` row names the marker and shed record; matches the CHANGELOG `[0.10.0]` behavioural note | closed |
| T-33-12 | Tampering | `.cargo/semver-checks-allowlist.toml` entries | high | mitigate | Plan 33-05 measured zero fired lints for the two `paladin-memory` rows (`RagRetrievalService`, `retrieve_context_with_timeout`), so no allowlist entry was written for them (both §9.2 rows are `N/A`); `make check-migration-allowlist` at HEAD reports "Allowlist is set-equal to the MIGRATION.md §9.2 deliberate-breaking register" | closed |
| T-33-13 | Repudiation | a behavioural change shipping without a release note | high | mitigate | CHANGELOG `[0.10.0]` section carries the RAG rationing bullet ("RAG retrieval output now carries a truncation marker and a shed record where it previously dropped silently (COMM-01, COMM-02, Phase 33)") plus the review-fix note for `RagRetrievalError::DuplicateMemoryId` (`c3327054`) | closed |
| T-33-14 | Tampering | public-surface baseline drift | medium | mitigate | `make api-surface` at HEAD: "API surface extracted (3959 items) — API surface unchanged" | closed |
| T-33-15 | Spoofing | an agent ticking a maintainer sign-off box | high | mitigate | `.project/v0.10.0/09-program-acceptance-audit.md` §11 (from line 1391) contains zero `- [x]` lines; all eight sign-off boxes in the file (seven Phase 29 boxes at lines 1261-1280 plus the new §11 box at line 1549) are unticked; `git diff <merge-base main>..HEAD` on the file is 166 insertions, 0 deletions (append-only, Phase 29 boxes byte-identical) | closed |
| T-33-16 | Repudiation | a gate written up as passing when it was not run | high | mitigate | §11 subsection "82% coverage floor — CI-attributed, not a local pass" names the CI `coverage` job and carries "Verdict: CI-ATTRIBUTED, not claimed as a local pass" with the reason the local run is not comparable | closed |
| T-33-17 | Tampering | quietly fixing a finding instead of recording it | medium | mitigate | §11 subsection "`cargo doc --workspace --no-deps` — carried condition, not a gate" records 73 warnings (up from 72 at §8) with "Verdict: PASS with one carried condition"; the finding is recorded, not fixed in place | closed |
| T-33-18 | Repudiation | an interrupted sweep read as complete | medium | accept | Every §11 gate row records head SHA `69500c9b` (11 occurrences plus the full 40-hex form) and date 2026-09-16, so drift against the final commit is visible. **Residual is now live:** HEAD `4f840f3f` is 15 commits past the sealed SHA, four of which (`eda96bbd` WR-01, `3ed8b9d2` WR-02, `6cd77888` WR-03, `3b332967` IN-01) changed executable Rust in `rag_retrieval_service.rs` (+131 lines) and `prelude.rs` after the re-seal. See AR-33-04 | closed |
| T-33-SC | Tampering | cargo dependency additions | low | accept | No new registry package: `proptest 1.4` was already resolved in `Cargo.lock` from the root crate and `paladin-llm` is a workspace path dependency (33-RESEARCH Package Legitimacy Audit: no `[ASSUMED]`/`[SUS]` rows). See AR-33-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-33-01 | T-33-SC | No new registry package entered `Cargo.lock`; the only additions are an already-resolved dev-dependency (`proptest 1.4`) and a workspace path crate. No legitimacy checkpoint was required per 33-RESEARCH. | Plan 33-01 threat model (plan-time disposition) | 2026-09-16 |
| AR-33-02 | T-33-08 | proptest shrink reports print generated random strings, never stored memory content, so a CI failure log cannot leak a real memory body. | Plan 33-03 threat model (plan-time disposition) | 2026-09-16 |
| AR-33-03 | T-33-10 | The "no other truncation site" claim is an identifier grep plus reviewed judgement, not a semantic proof. Flagged as an assumption in 33-04-PLAN/SUMMARY; a future truncation site introduced under a different identifier would not be caught by the sweep. | Plan 33-04 threat model (plan-time disposition) | 2026-09-16 |
| AR-33-04 | T-33-18 | §11 pins its gate results to SHA `69500c9b`; the mismatch against HEAD is visible by design. As of this audit the mismatch is real: four code-review fix commits changed `rag_retrieval_service.rs` and `prelude.rs` after the re-seal. Gates re-run at HEAD in this audit: `make api-surface` (unchanged) and `make check-migration-allowlist` (set-equal). The full Phase 29 gate list (semver-checks, MSRV, publish dry-run, clean-code, security, compat tests) was **not** re-run at HEAD here; the ADR-0051 tag-cut precondition should be judged against the sealed SHA or re-sealed on HEAD before tagging. | Plan 33-06 threat model (plan-time disposition); residual surfaced 2026-09-17 | 2026-09-16 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-17 | 19 | 19 | 0 | /gsd-secure-phase 33 (orchestrator L1 grep verification; short-circuit — plan-time register, ASVS L1, no auditor subagent spawned) |

Evidence commands run at HEAD `4f840f3f` on 2026-09-17: `cargo tree -p paladin-memory -i reqwest`,
`make check-migration-allowlist`, `make api-surface`, `git diff --stat $(git merge-base main HEAD) HEAD --
.project/v0.10.0/09-program-acceptance-audit.md`, plus file greps cited per row above.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-17
