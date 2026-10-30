# Handoff: Adopt Paladin `v0.10.1` in the Web3 Security Paladin consumer

**From:** Web3 Security Paladin ("web3sec"), the downstream consumer of `DF3NDR/paladin-dev-env`
**To:** the Paladin-dedicated agent, working in a standalone `paladin-dev-env` checkout
**Date:** 2026-10-08
**Purpose:** give you enough context to open a phase in the Paladin repo's own GSD roadmap and drive it to completion **without access to web3sec's planning files**. Everything you need is in this file plus its companion, `REFERENCE-adoption.diff`.

---

## 1. TL;DR

- web3sec consumes Paladin as a git submodule (`./paladin`) through path dependencies. Its committed pin is `35fd839`, a pre-`v0.10.0` development snapshot that is **631 commits behind `v0.10.1`** (`f7dae267`).
- A trial adoption has **already been done** on a throwaway copy, and `v0.10.1` works for web3sec:
  - **3 compile breaks, all deliberate and documented** in Paladin's `MIGRATION.md` §9.2.
  - **15 files, +39/−34 lines** of mechanical changes, all on the web3sec side.
  - After those changes, the whole web3sec workspace passes `cargo check --all-targets`, `cargo clippy -D warnings` and `cargo fmt --check`.
  - **`cargo test --workspace`: 2,592 passed, 0 failed, 2 ignored.** (Default features only; Docker- and live-gated tests were not run.)
- **No defect was found in Paladin `v0.10.1`.** Nothing in the framework has to change for web3sec to adopt it.
- What remains is (a) making the adoption real and reviewed, settling one semantic decision properly (§5, D-2) instead of using the trial's shortcut, (b) optionally hardening Paladin so the next downstream break is caught upstream (§6, workstream A), and (c) bumping the pointer.
- **Priority:** web3sec has frozen its in-flight work (invariant-driven detection and fuzz-to-prove) until this adoption lands.

---

## 2. Background you need

**What web3sec is.** A Rust, Paladin-orchestrated, multi-agent Solidity/EVM smart-contract audit system. It runs specialist agents (recon, static analysis, invariant/fuzz, storage layout, deductive reasoning, an adversarial "Tribunal" judge, a PoC "Prover") over a target repo and proves candidate findings with sandboxed Foundry PoCs. It is single-operator and CLI-first. It is **Paladin's only current consumer.**

**How web3sec consumes Paladin.** The `paladin` directory is a git submodule (`https://github.com/DF3NDR/paladin-dev-env.git`). It is **excluded** from web3sec's cargo workspace and consumed through three path dependencies in web3sec's root `Cargo.toml`:

```toml
paladin-ai     = { path = "paladin" }                        # facade; audit-agents enables features ["llm-deepseek", "llm-anthropic"]
paladin-ports  = { path = "paladin/crates/paladin-ports" }   # PaladinPort, LlmPort types, TokenCounterPort (not re-exported by the facade)
paladin-memory = { path = "paladin/crates/paladin-memory" }  # HeuristicTokenCounter (not re-exported by the facade)
```

web3sec crates that depend on Paladin: `audit-agents` (heaviest), `audit-cli`, `audit-arsenal`, `audit-invariants`, `audit-mcp`. Appendix A lists the consumed API surface.

**Your framework's own guidance on this upgrade.** Paladin's `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` §5 is binding context here:

1. **Clean break, no shims.** Paladin is pre-1.0 with one coordinated consumer, so the `Commissary::new` exactness-argument removal and the token-carrier changes landed outright. **Do not add compatibility shims to Paladin** to make the bump easier.
2. **Coordinated pointer bump.** web3sec stays on its old commit and adopts the whole release in one step. This handoff is that step.
3. **`Treasurer` guardrail.** `Treasurer` is reserved as a framework word. web3sec has a benchmark fixture domain term (`GarrisonTreasury`, `appointQuartermaster` in test fixtures). Do not conflate the two.
4. **Do not rewrite `.planning/` history** in either repo.

---

## 3. Current state (verified 2026-10-07/08)

| Item | Value | Note |
|---|---|---|
| Target tag | `v0.10.1` → `f7dae267` ("Merge pull request #56 … release/v0.10.1"), 2026-09-21 | workspace `version = "0.10.1"`, edition 2024, `rust-version = "1.88"`, Paladin toolchain pin 1.97.1 |
| web3sec committed pin | `35fd839` ("test(paladin-llm): port Commissary unit tests + wire module + facade export") | an ancestor of `v0.10.1`; its tree already said `version = "0.10.0"` (pre-tag) |
| web3sec submodule working tree | `4b7bed87` on branch `feature/v0.10.0-Commisary-Treasurer-plan` (pushed to `origin`) | 1 commit not in `v0.10.1`; the pointer change `35fd839 → 4b7bed87` is **uncommitted** in web3sec |
| That local-only commit | "chore: web3sec pre-plan Commissary-Treasurer changes" | adds only 7 planning docs under `.project/Milestone_13-Token-Economy/` and `.project/Milestone_14-Treasurer/`. **All 7 exist with identical content in `v0.10.1`** (`git diff 4b7bed87 v0.10.1 -- <those paths>` is empty) |
| Paladin working tree | clean (no uncommitted changes) | — |
| Toolchain | web3sec pins `rust-toolchain.toml` 1.95.0 (devcontainer `rust:1.95-slim-bookworm`); Paladin pins 1.97.1 | **Not a blocker:** the trial built `v0.10.1` with web3sec's 1.95.0, above Paladin's 1.88 MSRV |
| Shared third-party pins | `axum 0.8.4`, `reqwest 0.12.4`, `rmcp =2.1.0`, `tokio 1`, `sqlx 0.8` | unchanged between `35fd839` and `v0.10.1`, so no dependency drift for web3sec |

---

## 4. The three breaking changes that hit web3sec

All three are already registered in `MIGRATION.md` §9.2 at `v0.10.1`.

### B-1 `Commissary::new` / `Commissary::from_port` lost the `is_exact_counter: bool` parameter
- Now: `new(provider, capabilities, counter, config)` and `from_port(llm, counter, config)`. Exactness is read live from `TokenCounterPort::is_exact()`.
- web3sec always passed `false` and always uses `paladin_memory::token_counter::HeuristicTokenCounter`, which does **not** override `is_exact()`. It inherits the trait default `false`, which `v0.10.1`'s `heuristic_is_exact_reports_false_through_the_trait_default` test pins. **The change preserves behavior.** Fix: delete the argument.
- Sites: `audit-agents/src/{deductive.rs (from_port + a test ::new), breadth.rs, design_doc_enrichment.rs, fuzz.rs, tribunal.rs}`. Five comments elsewhere still mention `is_exact_counter`; update that prose.

### B-2 `PaladinResult.token_count: u32` → `PaladinResult.usage: TokenUsage`
- web3sec builds a `PaladinResult` with a full struct literal in `audit-agents/src/phalanx_dispatch.rs` (its `PaladinPort` implementation, which dispatches to specialist "routines"). It also reads `.token_count` in a unit test (`assert_eq!(result.token_count, 4242)`).
- **The semantic decision (D-2) is here.** web3sec's own `RoutineReport.token_count` is a **total-only** `u32`. `v0.10.1` has no total-only `TokenUsage` constructor (only `TokenUsage::new(prompt, completion)`), and `TokenUsage` documents an invariant that `total_tokens == prompt_tokens + completion_tokens`.
- **Persistence caveat.** Per MIGRATION §9.4 (D-25), a `PaladinResult` JSON persisted with the retired `token_count` key deserializes to `usage == TokenUsage::default()` (all zeros), and nothing maps the old key forward. The trial found no web3sec code that serializes or persists `PaladinResult` (only its own `RoutineReport`/registry types), but confirm this.

### B-3 `TokenUsage` gained three `Option<u32>` fields
- The new fields are `cache_read_tokens`, `cache_write_tokens` and `reasoning_tokens`. The struct is deliberately **not** `#[non_exhaustive]`, and its rustdoc recommends `..Default::default()`.
- web3sec builds `TokenUsage { prompt_tokens, completion_tokens, total_tokens }` literals in **22 places**, almost all in mock `LlmPort` responses in unit tests. Files: `audit-agents/src/{fuzz.rs ×5, scrivener.rs ×2, tribunal.rs, triage.rs, tally_stick.rs, sapper.rs, provider_breaker.rs, prover.rs, deductive.rs, design_doc_enrichment.rs}`, `audit-agents/tests/phase47_support/mod.rs`, `audit-invariants/src/armory.rs`, `audit-cli/src/submit_render.rs`.
- Fix: add `..Default::default()` (the new fields become `None`, meaning "provider did not report"), or use `TokenUsage::new(p, c)` where the total matches.

**Checked and not affecting web3sec:**
- v0.10.0's documented behavior changes:
  - M-B-01 `EdgeCondition::Custom` now fails closed. web3sec uses no `EdgeCondition`.
  - M-B-02 graceful shutdown. web3sec runs no Paladin server or engine binary.
  - M-B-03 tool-error redaction. This changes only text fed back to the model.
  - M-B-04 `WarEngine` waypoints. web3sec uses the legacy `PhalanxExecutionService`, which is explicitly unaffected.
- `#[non_exhaustive]` on `PaladinError`/`LlmError`/`BattalionError`/`StopReason`. web3sec has no exhaustive `match` on any of them.
- `served_by`. web3sec already sets `served_by: None`.
- The legacy `TokenCounter`/`TokenCounterFactory` removal and the `RagRetrievalService` change. web3sec uses neither.

---

## 5. Decisions to make (with recommendations)

| ID | Decision | Recommendation |
|---|---|---|
| **D-1** | What to do with the submodule's local-only commit `4b7bed87` | **Superseded; discard it.** Its 7 files are byte-identical in `v0.10.1` and the branch is already on `origin`. Pin web3sec **exactly** on tag `v0.10.1` (`f7dae267`), detached. Do not merge the feature branch. |
| **D-2** | How to map web3sec's total-only `RoutineReport.token_count` into `PaladinResult.usage` | **Preferred: carry a full `TokenUsage` through `RoutineReport`** (the fuzz and deductive outcomes accumulate per-call `LlmResponse.usage` upstream). That is lossless and matches Paladin's Milestone 13 Epic 2 intent. **Acceptable fallback:** `TokenUsage { total_tokens: report.token_count, ..Default::default() }`, which is what the trial diff does. It compiles and passes, but **breaks `TokenUsage`'s documented `total == prompt + completion` invariant**. If you pick the fallback, put a comment at the site saying so and record it as tech debt. Either way, no Paladin change is needed. |
| **D-3** | Toolchain alignment | **Leave web3sec on 1.95.0 for this phase** (proven to build `v0.10.1`). Optionally fix the stale web3sec Dockerfile comment "Rust pinned to match paladin's image" (Paladin is on 1.97.1). Moving web3sec to 1.97.1 is a separate, optional follow-up. |
| **D-4** | Add an upstream guard for the downstream API surface? | **Recommended (workstream A).** All three breaks were documented, yet nothing in Paladin *executes* the consumer's surface. A compile-only contract test covering Appendix A's imports would turn the next downstream break into a red Paladin CI check rather than a surprise at bump time. Keep it to symbols web3sec actually imports; don't freeze the whole API. |

---

## 6. Proposed phase for the Paladin roadmap

Use the Paladin repo's normal GSD flow (`/gsd-phase` to add it, then discuss, plan, execute). Number it from that roadmap. Medieval-military vocabulary is mandatory in Paladin code and docs.

**Suggested title:** "Downstream Adoption of v0.10.1 (web3sec Consumer Bump)"

**Goal:** Web3 Security Paladin, Paladin's only consumer, is moved from its pre-`v0.10.0` pin (`35fd839`) onto the `v0.10.1` release tag in one coordinated step, per Milestone 13 §5. The three documented breaks (B-1, B-2, B-3) are adopted on the consumer side with behavior preserved and no Paladin shims. Paladin gains a compile-time guard over the consumer-imported surface so future breaks surface upstream first.

**Requirements (suggested IDs; rename to the repo's convention):**
- **ADOPT-01:** The consumer pins `paladin` at exactly `v0.10.1` (`f7dae267`). The local-only `4b7bed87` is recorded as superseded (D-1).
- **ADOPT-02:** Consumer call sites are adapted for B-1, B-2 and B-3, with the D-2 token-usage mapping decided and documented (lossless preferred).
- **ADOPT-03:** The consumer workspace is green on the new pin: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo check --workspace`, `cargo test --workspace` (baseline: 2,592 pass, 0 fail, 2 ignored), with no `unwrap`/`expect`/`panic!` added to library code.
- **ADOPT-04 (D-4, Paladin-side):** a compile-only "consumer surface" contract test in Paladin covers every Appendix A import path and signature, wired into the existing gates (`make check-gates` / CI).
- **ADOPT-05 (optional):** Paladin's `MIGRATION.md` §9.8 upgrade checklist gains any consumer-facing note this adoption exposed. Candidate: the D-2 "total-only usage" pitfall, given there is no `from_total` constructor.

**Success criteria:**
1. `git -C paladin rev-parse HEAD` in the consumer equals `f7dae267…`, and the consumer commit records the pointer.
2. The consumer gates in ADOPT-03 pass, and the test count is ≥ the 2,592 baseline with 0 failures.
3. `grep -rn 'is_exact_counter\|token_count: report' crates/` in the consumer finds no stale call sites, and remaining prose mentions are updated.
4. The Paladin consumer-surface test passes on `v0.10.1`. Proven red: it fails to compile if (for example) a `bool` is re-added to `Commissary::new`.
5. No Paladin library API is changed to accommodate the consumer (no shims).

**Suggested plans:**
1. **Paladin-side guard (A):** add the consumer-surface contract test plus gate wiring, and optionally the MIGRATION §9.8 note. TDD: write the test against `v0.10.1`, then prove it red against a deliberately broken signature.
2. **Consumer adoption (B):** apply the adaptation (start from `REFERENCE-adoption.diff`, but implement D-2 properly), run the full consumer gates, and bump the pointer.
3. **Verification and return:** produce the return report (§9).

---

## 7. Workstream B: consumer adoption steps

Do this in a web3sec checkout if you have one. If you don't, hand it back with this section as the spec. Do not commit to web3sec from a standalone Paladin checkout.

```bash
# in the web3sec repo root (cargo workspace; "paladin" is the submodule)
git -C paladin fetch --tags
git -C paladin checkout --detach v0.10.1          # D-1: exact tag, not the feature branch
git -C paladin rev-parse HEAD                      # expect f7dae267...
# apply B-1/B-2/B-3 (see REFERENCE-adoption.diff; implement D-2 properly)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo test --workspace                             # expect >= 2592 passed, 0 failed
git add paladin <changed crates>                   # one commit: pointer + adoption together
```

Notes:
- `make clean-code` in web3sec is `fmt + clippy -D warnings + check`, and `make test` is `cargo test --workspace`.
- Cargo resolves the new Paladin crate versions and **`Cargo.lock` changes**; commit it with the bump.
- Some tests and fixtures use `include_str!` against the consumer's `fixtures/` and `benchmarks/` directories. The trial initially failed only because those directories weren't copied, so run the gates from a full checkout.
- Feature-gated suites (`docker-it` and live-LLM/fork tests) were **not** run in the trial. If your environment has Docker with gVisor (`runsc`) and API keys, an optional hermetic smoke run confirms the pipeline end to end, but it isn't required for this phase.

---

## 8. The reference diff (`REFERENCE-adoption.diff`)

- Unified diff from web3sec's current `crates/` (on pin `35fd839`) to the trial copy that builds and passes on `v0.10.1`. It covers 15 files (+39/−34) and is `cargo fmt`-clean.
- **Generated mechanically and not reviewed.** Treat it as a map of every site to change, not as the final patch. In particular, `phalanx_dispatch.rs` uses the D-2 **fallback** (`TokenUsage { total_tokens: report.token_count, ..Default::default() }`). Replace that if you take the preferred lossless route.
- Files: `audit-agents/src/{breadth, deductive, design_doc_enrichment, fuzz, phalanx_dispatch, prover, provider_breaker, sapper, scrivener, tally_stick, triage, tribunal}.rs`, `audit-agents/tests/phase47_support/mod.rs`, `audit-cli/src/submit_render.rs`, `audit-invariants/src/armory.rs`.

---

## 9. Return contract (what web3sec needs back)

web3sec closes its adoption work, and its frozen detection and fuzz-to-prove work resumes, when you report:
1. The consumer commit SHA containing the pointer bump to `f7dae267` (or, if you could not touch the consumer, the Paladin-side commits plus confirmation that §7 is still accurate).
2. The final D-2 choice, and which web3sec types changed if you went lossless (`RoutineReport`, the outcome types).
3. Gate output: fmt, clippy, check, and the test totals (pass/fail/ignored) against the 2,592/0/2 baseline.
4. The Paladin commits for ADOPT-04/05 and the tag or branch they live on. Say whether a `v0.10.2` is warranted; the expectation is no, because no library change is needed.
5. Anything that diverged from this handoff.

---

## 10. Constraints and conventions

- Hexagonal architecture; dependencies point inward. web3sec adapters implement Paladin ports and never the reverse.
- TDD; unit coverage ≥80% and integration ≥70%; `make clean-code` before commits; no `unwrap`/`expect`/`panic!` in library code.
- Medieval-military ubiquitous language in all new code and docs (e.g. "consumer-surface muster/roll", not "compat check").
- Every Paladin breaking change needs a MIGRATION §9.2 entry and a semver-checks allowlist update. This phase should introduce **none**.
- Do not rename `Commissary`, `TokenBudget`, `TokenCounterPort` or `TokenUsage`. Do not reintroduce `Quartermaster`. Keep `Treasurer` strictly a framework word (Milestone 14 is deferred).

## 11. Out of scope

- Milestone 14 (Treasurer spend governance) and any further token-economy work.
- web3sec feature work (invariant-driven detection, fuzz-to-prove) and its benchmark campaigns.
- Moving web3sec's toolchain to 1.97.1 (D-3).
- Cutting a new Paladin release, unless step 4 of §9 surfaces a real library defect.

---

## Appendix A: Paladin surface web3sec imports (as of pin `35fd839`)

All of these resolve on `v0.10.1`; the trial compiled every target. Candidates for the ADOPT-04 contract test.

**Facade `paladin::` (`paladin-ai`, features `llm-deepseek`, `llm-anthropic`):**
- `prelude::{LlmPort, LlmRequest, LlmResponse, LlmError (incl. ::Timeout, ::NetworkError), ArsenalPort, Armament, BattalionConfig, Phalanx, PaladinError, PaladinResult, StopReason}`
- `{Commissary, CommissaryError, CommissaryPlan, Consignment, ConsignmentItem}`
- `{AnthropicAdapter, AnthropicConfig, DeepSeekAdapter, DeepSeekConfig}` (incl. `DeepSeekConfig::from_env`)
- `{MockLlmAdapter, MultiStepMockLlmPort}` (test support)
- `core::platform::container::prompt::{PromptItem, PromptType, UserPrompt}`
- `core::platform::container::arsenal::{ArmamentCall, ArmamentResult, ArsenalError}`
- `core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus}`
- `core::platform::container::battalion::{BattalionError, ErrorStrategy, NodeError}`
- `core::base::entity::node::Node`
- `application::services::battalion::phalanx_service::PhalanxExecutionService`
- `application::services::paladin::paladin_execution_service::PaladinExecutionService`
- `infrastructure::resilience::circuit_breaker::CircuitBreaker`
- `infrastructure::adapters::arsenal::mcp_protocol::{MCPClient, looks_like_auth_failure}`

**`paladin_ports::` (direct path dep):**
- `output::llm_port::{LlmError, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse, TokenUsage, FinishReason}`
- `output::paladin_port::{PaladinPort, PaladinStream}` (web3sec **implements** `PaladinPort`)
- `output::token_counter_port::TokenCounterPort`

**`paladin_memory::` (direct path dep):**
- `token_counter::HeuristicTokenCounter`

**web3sec implements these Paladin traits:** `LlmPort` (several decorators and mocks, e.g. a provider circuit breaker and a tally decorator), `PaladinPort` (`phalanx_dispatch.rs`) and `ArsenalPort`.
