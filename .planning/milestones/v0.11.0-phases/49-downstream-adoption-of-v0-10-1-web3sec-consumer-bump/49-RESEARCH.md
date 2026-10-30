# Phase 49: Downstream Adoption of v0.10.1 (web3sec Consumer Bump) - Research

**Researched:** 2026-10-14 (tree `0afec211`, 184 commits past `v0.10.1`, workspace `version = "0.10.1"`)
**Domain:** downstream-consumer contract testing (a compile-only integration test over a Rust facade crate), release-note authoring (`MIGRATION.md`), and a consumer-side adoption spec for a repository that is not reachable from this checkout
**Confidence:** HIGH for every Paladin-side finding (all measured on this tree, several by compiling a draft muster); N/A for ADOPT-01..03 (consumer-side; closable only by the handoff §9 return report)

<user_constraints>
## User Constraints (from CONTEXT.md)

**No `49-CONTEXT.md` exists.** `/gsd-discuss-phase 49` was not run. The constraints below are taken verbatim from the filed handoff (`49-HANDOFF-web3sec.md`) and the ROADMAP Phase 49 section, as instructed. Where the handoff calls a point a "recommendation" (§5) the orchestrator has made it binding; where the ROADMAP says a point is "settled at discuss" (the muster's shape) nothing was settled, so it is treated as **Claude's Discretion** below.

### Locked Decisions

**Handoff §5 (decisions D-1..D-4), verbatim:**

| ID | Decision | Recommendation |
|---|---|---|
| **D-1** | What to do with the submodule's local-only commit `4b7bed87` | **Superseded; discard it.** Its 7 files are byte-identical in `v0.10.1` and the branch is already on `origin`. Pin web3sec **exactly** on tag `v0.10.1` (`f7dae267`), detached. Do not merge the feature branch. |
| **D-2** | How to map web3sec's total-only `RoutineReport.token_count` into `PaladinResult.usage` | **Preferred: carry a full `TokenUsage` through `RoutineReport`** (the fuzz and deductive outcomes accumulate per-call `LlmResponse.usage` upstream). That is lossless and matches Paladin's Milestone 13 Epic 2 intent. **Acceptable fallback:** `TokenUsage { total_tokens: report.token_count, ..Default::default() }`, which is what the trial diff does. It compiles and passes, but **breaks `TokenUsage`'s documented `total == prompt + completion` invariant**. If you pick the fallback, put a comment at the site saying so and record it as tech debt. Either way, no Paladin change is needed. |
| **D-3** | Toolchain alignment | **Leave web3sec on 1.95.0 for this phase** (proven to build `v0.10.1`). Optionally fix the stale web3sec Dockerfile comment "Rust pinned to match paladin's image" (Paladin is on 1.97.1). Moving web3sec to 1.97.1 is a separate, optional follow-up. |
| **D-4** | Add an upstream guard for the downstream API surface? | **Recommended (workstream A).** All three breaks were documented, yet nothing in Paladin *executes* the consumer's surface. A compile-only contract test covering Appendix A's imports would turn the next downstream break into a red Paladin CI check rather than a surprise at bump time. Keep it to symbols web3sec actually imports; don't freeze the whole API. |

**Handoff §2, "Your framework's own guidance on this upgrade" (Milestone 13 §5, binding), verbatim:**

1. **Clean break, no shims.** Paladin is pre-1.0 with one coordinated consumer, so the `Commissary::new` exactness-argument removal and the token-carrier changes landed outright. **Do not add compatibility shims to Paladin** to make the bump easier.
2. **Coordinated pointer bump.** web3sec stays on its old commit and adopts the whole release in one step. This handoff is that step.
3. **`Treasurer` guardrail.** `Treasurer` is reserved as a framework word. web3sec has a benchmark fixture domain term (`GarrisonTreasury`, `appointQuartermaster` in test fixtures). Do not conflate the two.
4. **Do not rewrite `.planning/` history** in either repo.

**Handoff §10 (constraints and conventions), verbatim:**

- Hexagonal architecture; dependencies point inward. web3sec adapters implement Paladin ports and never the reverse.
- TDD; unit coverage ≥80% and integration ≥70%; `make clean-code` before commits; no `unwrap`/`expect`/`panic!` in library code.
- Medieval-military ubiquitous language in all new code and docs (e.g. "consumer-surface muster/roll", not "compat check").
- Every Paladin breaking change needs a MIGRATION §9.2 entry and a semver-checks allowlist update. This phase should introduce **none**.
- Do not rename `Commissary`, `TokenBudget`, `TokenCounterPort` or `TokenUsage`. Do not reintroduce `Quartermaster`. Keep `Treasurer` strictly a framework word (Milestone 14 is deferred).

(Note: the handoff's "unit ≥80% / integration ≥70%" is superseded by ADR-0006 — one floor, **82 % workspace line coverage** — per CLAUDE.md and the 2026-08-13 correction in `.github/copilot-instructions.md`.)

**ROADMAP Phase 49 "Ground truth" paragraph (verified 2026-10-14 on `e8815625`), verbatim:**

> B-1, B-2 and B-3 read exactly as the handoff states — `Commissary::new(provider, capabilities, counter, config)` / `from_port(llm, counter, config)` in `crates/paladin-llm/src/services/commissary.rs`; `TokenUsage` (`paladin_core::platform::container::token_usage`, re-exported by `paladin_ports::output::llm_port`) has six fields, derives `Default`, is not `#[non_exhaustive]`, and offers `new(prompt, completion)` plus `with_cache_read` / `with_cache_write` / `with_reasoning` but **no total-only constructor**, so D-2's lossless route is the only one that keeps the documented `total == prompt + completion` invariant. **Drift the handoff cannot see:** the tree web3sec meets at its *next* bump already differs from `v0.10.1` on four Appendix-A items — `battalion::ErrorStrategy` and the legacy `battalion::NodeError` summary are removed (Phase 44, §9.2 rows; the structured `platform::container::node_error::NodeError` remains), `core::base::entity::node::Node` is removed in favour of `base::entity::Entity<T>` with payload `data` (Phase 48, §9.2 row), and `PaladinResult` gained `cost: Option<Cost>` (Phase 38, `#[serde(default)]`), which breaks web3sec's full struct literal in `phalanx_dispatch.rs` again. Appendix A also lists `mcp_protocol::looks_like_auth_failure`, which is a private `fn` at both `v0.10.1` and `HEAD` (its rustdoc says it was ported *from* web3sec's `audit-mcp`), so the imported surface must be re-verified from the consumer's actual `use` lines before the muster freezes it. Precedent for a repository-level guard: `tests/legacy_removal_guard.rs`, `tests/entity_rename_guard.rs`, `tests/treasurer_vocabulary_guard.rs` (std-only scans); the muster is a different shape — a compile-time `use` + fn-pointer test — settled at discuss. `MIGRATION.md` §9.8 is today a v0.9 → v0.10 *operator* checklist with no library-consumer section.

**ROADMAP goal sentence, verbatim:** "No Paladin library API changes to accommodate the consumer." And success criterion 5: "**no Paladin library API is changed, shimmed or deprecated** for the consumer: zero new §9.2 rows and zero `semver-checks` allowlist changes from this phase; `make clean-code`, `make check-gates`, `make api-surface` and the 82 % coverage floor stay green. No `v0.10.2` is cut."

**Handoff §11 (out of scope), verbatim:**

- Milestone 14 (Treasurer spend governance) and any further token-economy work.
- web3sec feature work (invariant-driven detection, fuzz-to-prove) and its benchmark campaigns.
- Moving web3sec's toolchain to 1.97.1 (D-3).
- Cutting a new Paladin release, unless step 4 of §9 surfaces a real library defect.

### Claude's Discretion

- The **mechanics of the muster** (ROADMAP: "a compile-time `use` + fn-pointer test — settled at discuss"; no discuss ran): file name, pin classes, `[[test]]` declaration, gate wiring. Recommended below.
- The **placement and shape** of the `MIGRATION.md` consumer note (new `###` under §9.8 vs a new `## 9.9`).
- How the plan **represents the unreachable consumer work** (ADOPT-01..03) honestly.
- Whether to add a convenience `make check-consumer-surface` alias.

### Deferred Ideas (OUT OF SCOPE)

No CONTEXT.md, so no `## Deferred Ideas` section exists; the handoff §11 list above is the out-of-scope set. Additionally out of scope by ROADMAP sequencing: Phases 46/47 (docs pass and release) consume this phase's outputs; Phases 50/51 change surface the muster will then guard.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ADOPT-01 | Consumer pins `paladin` at exactly `v0.10.1` (`f7dae267`), detached, one commit with adaptation + `Cargo.lock`; local-only `4b7bed87` recorded as superseded | **Consumer-side; not executable here.** Paladin-side facts verified: `v0.10.1` is an *annotated* tag (tag object `7593ab4d…`, commit `f7dae2676580786e9541fefee8f787623b46c70f`); `4b7bed87` exists in this clone (on `origin/feature/v0.10.0-Commisary-Treasurer-plan`), is **not** an ancestor of `v0.10.1`, and `git diff --quiet 4b7bed87 v0.10.1 -- <its 7 paths>` exits 0 (byte-identical). Deliverable: consumer spec + return-report template (§"Consumer-side deliverable"). |
| ADOPT-02 | Call sites adapted for B-1/B-2/B-3; D-2 decided and documented | **Consumer-side.** Paladin supplies the precise lossless D-2 design: `TokenUsage` already has saturating `Add`/`AddAssign`/`Sum` at `v0.10.1` (no new helper needed); the fallback shape is not additive under Paladin's own `Add`. |
| ADOPT-03 | Consumer gates green on the new pin (≥2,592 / 0 / 2) | **Consumer-side.** Only the §9 return report can close it. Paladin supplies the gate list, the `include_str!` caveat and a report-intake checklist. |
| ADOPT-04 | Compile-only consumer-surface muster, green on `cargo test`, wired into gates, proven red | **Fully researched and de-risked:** a draft muster was compiled (green, clippy-clean) on this tree and three mutation experiments run (§"Proven red"). |
| ADOPT-05 | `MIGRATION.md` consumer note (D-2 pitfall + `v0.10.1` → `v0.11.0` break list); no API/allowlist change | Break list **re-derived and larger than the ROADMAP's four items** (§"Drift register"); placement and guard constraints verified (`check-migration-allowlist.sh` only reads §9.2). |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **TDD** (red-green-refactor); coverage floor **82 % workspace line coverage** (ADR-0006, `cargo llvm-cov --fail-under-lines` in CI's `coverage` job). A compile-only test adds no instrumented library line (see Validation Architecture).
- **Dependencies flow inward only.** The muster is a facade-crate integration test (outermost ring, allowed to import every crate).
- **Ubiquitous language:** Medieval-military terms; `Entity<T>` is deliberately generic and "node" means only a WarGraph/Campaign vertex (ADR-0060). Use "muster"/"roll"; never "compat check".
- **No `unwrap()`/`expect()`/`panic!` in library code** (tests may). **Before committing a parent task:** `cargo test` → `cargo fmt --check` → `cargo clippy` → `make api-surface`, conventional commit, stop after each major task.
- **Security:** `make security` and `cargo clippy -- -D warnings`; manual credential-handling review; **do not reintroduce Snyk**; CodeQL is advisory-only.
- **Do not add attribution lines beyond the system reminder's** (commit trailer is supplied by the harness).
- All public APIs need doc tests — **N/A**: this phase adds no public API.

## Summary

Phase 49 has two halves with very different executability. The **Paladin half** (ADOPT-04, ADOPT-05) is small, mechanical and fully researchable here: add one integration-test file that `use`s every path and pins every signature web3sec imports, declare it with `required-features`, prove it red by mutation, and add a consumer note to `MIGRATION.md`. The **consumer half** (ADOPT-01..03) happens in a repository that is not reachable from this checkout; the handoff (§7, §9) itself says to hand it back as a spec when no checkout is available and to report "the Paladin-side commits plus confirmation that §7 is still accurate". The plan must therefore deliver a self-contained consumer spec and a return-report template, leave the consumer SHA / gate totals as consumer-filled fields, and **not mark ADOPT-01..03 complete** until the report is filed.

Re-verifying Appendix A against the tree produced four findings the ROADMAP does not state. (1) Two Appendix-A items do not resolve at HEAD for the reasons the ROADMAP gives, and one more does not resolve at all: `looks_like_auth_failure` is a private `fn` at `35fd839`, `v0.10.1` and HEAD, so web3sec cannot have imported it (it has its own copy in `audit-mcp`). (2) The `v0.10.1` → HEAD break list that the muster exposes is **roughly three times the ROADMAP's four items**: also `LlmResponse.cost` (Phase 38, breaks the 22 mock-response literals a *second* time), `LlmError::RateLimitExceeded` (unit → `#[non_exhaustive]` struct variant), `BattalionConfig` losing `timeout_seconds`/`retry_policy`/`error_strategy` (+ builders), `BattalionError::Timeout`, `PaladinError::LlmError(String)` + `is_retryable`/`is_terminal`, `PromptItem.node` → `.entity` and `ContentItem.node` → `.entity`. (3) The D-2 fallback is worse than the handoff states: `TokenUsage`'s saturating `Add` **recomputes** `total_tokens` from prompt + completion, so a fallback-shaped value `{total_tokens: n, ..Default}` contributes **0** to any `Add`/`Sum`. (4) `make check-gates` is a sub-second, script-only gate that no workflow runs; the real "existing gates" for a Rust test are `cargo test` (CI integration job, coverage job) **and** `cargo check/clippy --all-targets` (inside `make clean-code`), and the latter alone already enforces a compile-only muster.

A draft muster (278 lines, below) compiled first try on this tree, is clippy-clean with one documented `allow`, passes alongside the three precedent guard tests, and was proven red by re-adding a `bool` to `Commissary::new` with both non-test callers patched so that **only** the muster fails to compile (E0308 and E0061 at the muster's own lines).

**Primary recommendation:** write `tests/consumer_surface_muster.rs` (strict, no-`..` literals for the structs the consumer constructs; fn-pointer pins for constructors; minimal never-called implementations of `LlmPort`/`PaladinPort`/`ArsenalPort`), declare it as `[[test]] required-features = ["llm-anthropic", "llm-deepseek"]`, prove it red with the scripted mutation (error location must be the muster file and the only `could not compile` line must name the muster target), write the `MIGRATION.md` §9.8 `###` consumer note from the drift register, and ship the consumer spec + return-report template with ADOPT-01..03 left Pending behind an `autonomous: false` report-intake checkpoint.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Consumer-surface muster | Facade crate integration tests (`tests/`, package `paladin-ai`) | — | Sees the surface exactly as a path-dep consumer does (`paladin::…`) and can also `use` the direct path deps `paladin_ports` / `paladin_memory` the consumer declares |
| Gate wiring | CI / Makefile | `Cargo.toml` `[[test]]` | `cargo check/clippy --all-targets` (compile) and `cargo test --workspace` already reach a `tests/*.rs` target; no new job required |
| Consumer upgrade note | Docs (`MIGRATION.md` §9.8) | CHANGELOG `[Unreleased]` | Single register of upgrade guidance; the §9.2 table (machine-checked) is deliberately untouched |
| Consumer adoption (ADOPT-01..03) | External repo (web3sec) | Phase directory (spec + return report) | Not reachable; Paladin supplies spec/template only |
| Break-set derivation | Compiler (muster vs older tree) / §9.2 register | `git diff v0.10.1..HEAD` of surface files | The register is hand-written; the diff is the independent check |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust toolchain | 1.97.1 (`rust-toolchain.toml`; MSRV 1.88) | Compile the muster | Repo pin [VERIFIED: `rust-toolchain.toml`, `cargo --version` = 1.97.1] |
| `cargo test` / `cargo check --all-targets` | bundled | Run / compile-gate the muster | Existing gates (`make check`, `make lint`, CI) |
| `async-trait` | `0.1.88` (workspace) | `#[async_trait]` minimal trait impls inside the muster | Already a root `[dependencies]` entry and what the real traits use [VERIFIED: `Cargo.toml`] |
| `chrono`, `uuid`, `serde_json`, `futures` | workspace / root deps | Build `LlmResponse` / `ArmamentResult` literals and the `Stream` return type | Already root `[dependencies]`; integration tests may use a package's normal dependencies [VERIFIED: draft compiled] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `paladin_ports`, `paladin_memory` | path (0.10.1) | The consumer's two direct path deps | Muster `use`s them directly, mirroring web3sec's `Cargo.toml` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Plain `fn`-pointer / literal / impl pins | `trybuild` / `compile_fail` doctests | Neither exists in the repo (`Cargo.lock` has no `trybuild`; zero `compile_fail` occurrences) and a `compile_fail` doctest cannot live in an integration-test file. Not needed: the muster is *positive*; the *red proof* is a one-off scripted mutation. **Do not add a dependency.** |
| `[[test]] required-features` | `#[cfg(feature = …)]` on individual pins | `cfg` silently shrinks the muster under `--no-default-features`; `required-features` makes `cargo test --test consumer_surface_muster --no-default-features` fail loudly: `target … requires the features: llm-anthropic, llm-deepseek` [VERIFIED: ran] |

**Installation:** none. **No new packages.**

## Package Legitimacy Audit

This phase installs no external packages (`Cargo.toml` and `Cargo.lock` are untouched except for one `[[test]]` table in `Cargo.toml`). The seam (`gsd-tools query package-legitimacy check`) was therefore not invoked.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| — | — | — | — | — | — | No packages added |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## The Imported Surface, Re-Verified at HEAD

Facade = package `paladin-ai`, `[lib] name = "paladin"` [VERIFIED: `Cargo.toml`]. `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]`; web3sec's `paladin-ai = { path = "paladin" }` keeps defaults on, and `audit-agents` additionally names `llm-deepseek` / `llm-anthropic` (both exist at HEAD). `paladin-llm` is a facade dependency with `default-features = false, features = ["mock"]`, so `MockLlmAdapter` / `MultiStepMockLlmPort` are never feature-gated [VERIFIED: `Cargo.toml`, `src/lib.rs:192`].

"Δ" = change between `v0.10.1` and `0afec211`, from `git diff v0.10.1..HEAD` plus the §9.2 register.

### Facade `paladin::`

| Appendix-A item | Resolves at HEAD? | Exact path (evidence) | Feature gate | Δ since `v0.10.1` |
|---|---|---|---|---|
| `prelude::LlmPort`, `LlmRequest`, `LlmResponse`, `LlmError` | yes | `src/prelude.rs:43` re-exports `paladin_ports::output::llm_port::{…}` | none | `LlmResponse` gains `cost: Option<Cost>` (DR-6); `LlmError::RateLimitExceeded` changes shape (DR-7); `Timeout(String)` / `NetworkError(String)` unchanged |
| `prelude::ArsenalPort`, `Armament` | yes | `src/prelude.rs:41`, `:32` | none | none (`arsenal_port.rs`, `arsenal/core.rs` byte-identical) |
| `prelude::BattalionConfig` | yes | `src/prelude.rs` via crate root (`pub use core::platform::container::battalion::{BattalionConfig, BattalionError}`) | none | **fields `timeout_seconds`, `retry_policy`, `error_strategy` and builders removed; `aegis` added** (DR-8) |
| `prelude::Phalanx` | yes | `src/prelude.rs:38` | none | none to `Phalanx::new(Vec<Paladin>, BattalionConfig) -> Result<Phalanx, BattalionError>` |
| `prelude::PaladinError` | yes | `src/prelude.rs:51` → `paladin_core::…::paladin_error::PaladinError` (`#[non_exhaustive]`) | none | `LlmError(String)`, `is_retryable()`, `is_terminal()` removed (DR-10) |
| `prelude::PaladinResult`, `StopReason` | yes | `src/prelude.rs:44` | none | `PaladinResult.cost` (DR-5); `StopReason` (`#[non_exhaustive]`) gains `AllowanceHalted(AllowanceRefusal)` (safe: wildcard arm already required) |
| `Commissary`, `CommissaryError`, `CommissaryPlan`, `Consignment`, `ConsignmentItem` | yes | `src/lib.rs:200` | none | **byte-identical** (`commissary.rs` unchanged) |
| `AnthropicAdapter`, `AnthropicConfig` | yes | `src/lib.rs:186-187` | `llm-anthropic` | public shape unchanged (`AnthropicConfig` fields `api_key, base_url, model, max_tokens, timeout_seconds`; `new(String,String,String,u32)`, `from_env()`); adapter internals changed |
| `DeepSeekAdapter`, `DeepSeekConfig` (+ `from_env`) | yes | `src/lib.rs:189-190` | `llm-deepseek` | public shape unchanged (`DeepSeekConfig::from_env() -> Result<Self, String>`, `new(String,String,String)`) |
| `MockLlmAdapter`, `MultiStepMockLlmPort` | yes | `src/lib.rs:192` | none | no public change |
| `core::platform::container::prompt::{PromptItem, PromptType, UserPrompt}` | yes | `src/core/platform/mod.rs:33` re-exports the module | none | **`PromptItem.node` → `.entity`; payload read is `.entity.data.x` (was `.node.node.x`)** (DR-4) |
| `core::platform::container::arsenal::{ArmamentCall, ArmamentResult, ArsenalError}` | yes | `src/core/platform/mod.rs:49` | none | none |
| `core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus}` | yes | `src/core/platform/mod.rs:28` | none | `Paladin` is now `Entity<PaladinData>` (DR-3); `PaladinData` 13 fields, unchanged at HEAD (Phase 51 will add one — see Pitfall 9) |
| `core::platform::container::battalion::BattalionError` | yes | `src/core/platform/mod.rs` glob re-export of the core battalion module | none | `#[non_exhaustive]`; `Timeout` variant removed (DR-9) |
| `…::battalion::ErrorStrategy` | **NO — removed** | was `paladin_core::…::battalion::ErrorStrategy` | — | Phase 44 (§9.2 row, `enum_missing`). The Maneuver's `paladin_battalion::maneuver::ErrorStrategy` is a *different* item and still exists (`crates/paladin-battalion/src/maneuver/mod.rs:18`). Replacement: `Aegis.on_error` |
| `…::battalion::NodeError` (legacy summary) | **NO — removed** | — | — | Phase 44. Replacement is the structured `platform::container::node_error::NodeError` (`crates/paladin-core/src/platform/container/node_error.rs:152`), now the element of `BattalionResult.node_errors` |
| `core::base::entity::node::Node` | **NO — removed** | now `paladin::core::base::entity::Entity` (`src/core/mod.rs:109` `pub use paladin_core::base`) | — | Phase 48: payload field `node` → `data`; `Entity::new(data, name)` / `new_with_uuid` keep the old signatures |
| `application::services::battalion::phalanx_service::PhalanxExecutionService` | yes | `src/application/services/battalion/mod.rs:17` `pub use paladin_battalion::phalanx_service` | none | `new(Arc<dyn PaladinPort>)`, `execute(&self, &Phalanx, &str) -> Result<BattalionResult, BattalionError>` unchanged; behaviour changed (Aegis retry/timeout, M-B-06/07) |
| `application::services::paladin::paladin_execution_service::PaladinExecutionService` | yes | `src/application/services/paladin/mod.rs` | none | `new(Arc<dyn LlmPort>, Arc<CircuitBreaker>, Option<Arc<dyn GarrisonPort>>, Option<Arc<dyn ArsenalPort>>)` and `execute(&self, &Paladin, &str)` unchanged |
| `infrastructure::resilience::circuit_breaker::CircuitBreaker` | yes | `src/infrastructure/resilience/circuit_breaker.rs:121` | none | `new(u32, u32, Duration)`, `call`, `call_async` unchanged; now counts only `Transient` failures (M-B-10) |
| `infrastructure::adapters::arsenal::mcp_protocol::MCPClient` | yes | `src/infrastructure/adapters/arsenal/mcp_protocol.rs:62` | none | no public change (`connect_stdio`, `connect_streamable_http[_with_timeout]`, `server_capabilities`, `discover_tools`, `invoke_tool`) |
| `…::mcp_protocol::looks_like_auth_failure` | **NO — private `fn`** | `mcp_protocol.rs:365` (`fn looks_like_auth_failure(msg: &str) -> bool`, no `pub`) | — | Private at `35fd839`, `v0.10.1` **and** HEAD [VERIFIED: `git show <rev>:… \| grep`]. Its doc says it "Mirrors `crates/audit-mcp/src/etherscan.rs::looks_like_auth_failure`" — web3sec owns the original. **Appendix A is wrong here; do not pin.** |

### Direct path dependencies

| Appendix-A item | Resolves? | Path (evidence) | Δ |
|---|---|---|---|
| `paladin_ports::output::llm_port::{LlmError, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse, TokenUsage, FinishReason}` | yes | `crates/paladin-ports/src/output/llm_port.rs` (`TokenUsage` re-exported at `:1127`) | as in the facade rows; `StreamingResponse` is `#[non_exhaustive]` (gains `cost`, unconstructible by literal anyway) |
| `paladin_ports::output::paladin_port::{PaladinPort, PaladinStream}` | yes | `paladin_port.rs:798`, `:569` | **no new required trait method** (`execute`, `execute_stream`, `validate` required; `execute_observed`, `execute_scoped` defaulted) [VERIFIED: diff] |
| `paladin_ports::output::token_counter_port::TokenCounterPort` | yes | `token_counter_port.rs:77` (`count`, `name` required; `is_exact` defaulted `false`) | byte-identical |
| `paladin_memory::token_counter::HeuristicTokenCounter` | yes | `crates/paladin-memory/src/token_counter/heuristic.rs:17` (unit struct, `Default`) | byte-identical; `heuristic_is_exact_reports_false_through_the_trait_default` pins B-1's behaviour-preservation claim (`heuristic.rs:93`) |

**Traits web3sec implements** — `LlmPort` (6 required methods: `generate`, `generate_stream`, `validate_model`, `get_available_models`, `get_provider_name`, `get_capabilities`), `PaladinPort`, `ArsenalPort` (3 required: `list_armaments`, `invoke`, `validate_call`) — gained **no new required method** since `v0.10.1`, so a minimal implementation written today also compiles against `v0.10.1` for those three.

**Byte-identical since `v0.10.1`** (`git diff --quiet v0.10.1..HEAD` exit 0): `TokenUsage`, `Commissary`, both token counters, `TokenCounterPort`, the arsenal domain types, `ArsenalPort`. The B-1/B-2/B-3 adaptation therefore does **not** age; only the *new* breaks below do.

### Drift register (v0.10.1 → HEAD) — the break list the muster exposes

ROADMAP names DR-1, DR-2, DR-3, DR-5. The rest are additional and are what ADOPT-05's note must list. Every row is already in §9.2 (so **no new §9.2 row is needed** — the note cross-references them).

| ID | Break | §9.2 / §9.1 anchor | Hits Appendix-A item |
|---|---|---|---|
| DR-1 | `battalion::ErrorStrategy` removed → `aegis.on_error` | Phase 44 row `ErrorStrategy`; M-B-07, M-B-11 | `BattalionError` import group; `BattalionConfig::with_error_strategy` |
| DR-2 | legacy `battalion::NodeError` removed; `BattalionResult.node_errors: Vec<node_error::NodeError>` `{node_id, attempt, transience, source}` | rows `NodeError`, `BattalionResult` | any read of `.node_name` / `.error` |
| DR-3 | `Node<T>` → `Entity<T>`; payload `.node` → `.data`; `Paladin = Entity<PaladinData>` | row `Node`; M-B-12 | every `paladin.node.<field>` read, `Node::new` |
| DR-4 | `PromptItem.node` → `.entity` (payload `.entity.data`); `ContentItem.node` → `.entity` | rows `PromptItem`, `ContentItem` | `LlmRequest.prompt` / `.attachments` reads |
| DR-5 | `PaladinResult.cost: Option<Cost>` | row `PaladinResult` (Phase 38 extension) | full literal in `phalanx_dispatch.rs` (use `..Default::default()` / `PaladinResult::new`) |
| DR-6 | **`LlmResponse.cost: Option<Cost>`** | row `LlmResponse` (Phase 38-04) | the **22 mock-response literals** (they are all `LlmResponse { … usage: TokenUsage{…}, … }`) break a second time |
| DR-7 | **`LlmError::RateLimitExceeded` unit → `#[non_exhaustive]` struct variant `{retry_after, hints}`**; build with `LlmError::rate_limited(..)`, match with `{ .. }` | row `LlmError` (PACE-01) | any construction or unit-pattern match |
| DR-8 | **`BattalionConfig`: `timeout_seconds`, `retry_policy`, `error_strategy` and `with_timeout` / `with_retry_policy` / `with_error_strategy` removed; `aegis` + `with_aegis` added** | row `BattalionConfig`; M-B-06/07 | `BattalionConfig` + legacy `PhalanxExecutionService` |
| DR-9 | **`BattalionError::Timeout` removed** (timeouts surface as `BattalionError::Node` with a `Timeout` source) | row `BattalionError` | any `BattalionError::Timeout` arm |
| DR-10 | **`PaladinError::LlmError(String)`, `is_retryable()`, `is_terminal()` removed → `LlmFailure { transience, status, provider, message }` + `transience()`** | row `PaladinError` (Phase 44 removals); M-B-09/10 | any construction/match of the string variant |
| DR-11 | `StopReason` gains `CallLimit`, `TokenBudget`, `AllowanceHalted(..)` | row `StopReason` | none if matches already carry a wildcard (enum is `#[non_exhaustive]`) |

The handoff §4 states web3sec has no exhaustive `match` on `PaladinError`/`LlmError`/`BattalionError`/`StopReason`; that does **not** cover constructing `PaladinError::LlmError(..)` or `LlmError::RateLimitExceeded`, or reading removed `BattalionConfig` fields. Whether web3sec does any of that is unknowable from here — see Open Question 1.

## Architecture Patterns

### System Architecture Diagram

```
 web3sec repo (external, unreachable)                      Paladin repo (this checkout)
 ─────────────────────────────────────                     ──────────────────────────────────────────
 audit-agents / -cli / -arsenal /                          crates/paladin-{core,ports,llm,memory,battalion,…}
 -invariants / -mcp                                               │  (pub items the consumer imports)
   │ path deps: paladin-ai (facade),                              ▼
   │            paladin-ports, paladin-memory           facade `paladin` (src/lib.rs, prelude, core/, application/,
   ▼                                                              infrastructure/)  ── features llm-anthropic,
 submodule pin 35fd839 ──(ADOPT-01..03: spec only)──►   v0.10.1                       llm-deepseek (default-on)
                                                                  │
                                              tests/consumer_surface_muster.rs  ◄── ADOPT-04
                                                │  use paths  ─┐
                                                │  fn-pointer pins (Commissary::new/from_port, TokenUsage::*)
                                                │  struct-literal pins (TokenUsage, LlmResponse, PaladinResult,
                                                │                       PaladinData, ConsignmentItem)
                                                │  minimal impls of LlmPort / PaladinPort / ArsenalPort
                                                ▼
                                   `cargo check --all-targets`  (make check ⊂ make clean-code)
                                   `cargo clippy --all-targets --all-features -D warnings` (make lint, CI)
                                   `cargo test --workspace --features integration-tests` (CI integration job)
                                   `cargo llvm-cov … --features integration-tests,llm-all` (CI coverage job)
                                                │
                              red build  ⇒ a downstream break reached Paladin first
                              fix = update the pin + the MIGRATION §9.2 row in the SAME commit

 MIGRATION.md §9.8 `### Library consumers` (ADOPT-05) ◄── drift register DR-1..DR-11 + D-2 pitfall
   (check-migration-allowlist.sh reads ONLY §9.2; entity/legacy guards skip MIGRATION.md by name)
```

### Recommended Project Structure
```
tests/
└── consumer_surface_muster.rs        # NEW — the muster (auto-discovered AND declared in Cargo.toml)
Cargo.toml                            # +1 [[test]] table (name = file stem, required-features)
MIGRATION.md                          # +1 `###` subsection under §9.8 (NOT inside §9.2)
CHANGELOG.md                          # +1 [Unreleased] entry (Added: muster; Documentation: consumer note)
Makefile                              # OPTIONAL: check-consumer-surface alias (NOT in check-gates)
.planning/phases/49-…/
├── 49-CONSUMER-ADOPTION-SPEC.md      # NEW — handoff §7 made self-contained + D-2 design + pre-flight greps
├── 49-RETURN-report.md               # NEW — §9 template, Paladin-side facts filled, consumer fields PENDING
└── 49-MUSTER-RED-EVIDENCE.md         # NEW — mutation transcripts (exit codes, error locations)
```

### Pattern 1: Pin classes (each catches a different break)

| Pin class | Catches | Mechanism |
|---|---|---|
| `use` of every path | removed / moved / feature-gated item (E0432) | the import itself |
| `let _: fn(A,B)->R = Type::method;` | parameter added/removed/retyped (E0308) — the B-1 class | coercion of the fn item |
| real call with literal args (`Commissary::new("literal-str", …)`) | an `impl Into<String>` parameter narrowing (E0277), arity (E0061) | a call in a never-executed fn |
| `async fn` body that `.await`s the call into a typed `let` | async method signature/return changes (fn pointers cannot name `impl Future`) | never-called `async fn` |
| full struct literal, **no `..`** | added/removed/renamed field (E0063 / E0560) — the B-2/B-3/DR-5/DR-6 class | literal of the structs the consumer constructs |
| `..Default::default()` literal | removal of `Default` (the consumer's preferred insulation) | literal with functional update |
| minimal `impl Trait for Stub` with **only the required methods** | new required trait method (E0046), changed signature (E0053), loss of object safety (`Arc<dyn Trait>` coercion) | stub + `Arc<dyn Trait>` |
| identity fn `fn(a::T) -> b::T { x }` | two paths ceasing to name one type (facade prelude vs `paladin_ports`) | type-equality via return |

**Strictness policy (state it in the file header):** pin the structs the consumer *constructs* as strict literals. A red from an *additive* `#[serde(default)]` field is **intended** — that is exactly DR-5/DR-6; the fix is "update the pin and its §9.2 row in the same commit". Use `..Default::default()` only where the handoff says the consumer does (`CommissaryPlan`, `TokenUsage` after the bump).

### Pattern 2: Never execute a pin
Every pin lives in a function or `async fn` that nothing calls, plus **one** trivial `#[test]` so `cargo test` reports a passing target. Executing the pins would spawn a process (`MCPClient::connect_stdio`), read the environment (`DeepSeekConfig::from_env`) and build HTTP clients. Compile-only is also the security property (see Security Domain).

### Pattern 3: Declaration and gate wiring (answers orchestrator Q2)

- **Declare it.** Add, beside the other `[[test]]` tables:
  ```toml
  [[test]]
  name = "consumer_surface_muster"
  path = "tests/consumer_surface_muster.rs"
  required-features = ["llm-anthropic", "llm-deepseek"]
  ```
  Precedent for explicit `[[test]]` + `required-features`: `vision_integration`, `cli`, `e2e_platform_api`, `v0_9_config_boot` [VERIFIED: `Cargo.toml`]. The three guard tests (`legacy_removal_guard`, `entity_rename_guard`, `treasurer_vocabulary_guard`) are **not** declared — they rely on auto-discovery — but they have no features to require. With `name` equal to the file stem Cargo produces **one** target (no duplicate) [VERIFIED: `cargo metadata` after a trial edit, reverted].
- **Does the default `cargo test` run it?** Yes: both features are in `default`. It also runs in CI's broad suite (`cargo test --workspace --features integration-tests --verbose -- --test-threads=1`, `ci.yml:828`) and the coverage job (`scripts/coverage.sh`, `--features integration-tests,llm-all`). It does **not** run under `make test` (`--workspace --lib --bins`) or `make test-all`/`make test-integration` (name-filtered runs in `scripts/run_integration_tests.sh`) — same as the three guards.
- **Compile-only means `check` is the gate.** `make clean-code` = `fmt lint lint-shell check doc-check check-api-examples`; `lint` is `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `check` is `cargo check --workspace --all-targets`. Both compile every integration-test target whose required features are satisfied, so a red muster fails `make clean-code` **without running a single test**. CI runs the same clippy (`ci.yml:60`) and `cargo check --workspace --all-features --all-targets` (`ci.yml:305`).
- **`make check-gates` is the wrong place.** It is eight offline *script* guards that finish in ~1 s and **no workflow invokes it** (`grep` over `.github/workflows`, `Makefile`, `.pre-commit-config.yaml`: only its own definition). Putting a cargo compile in it would change its character; the Makefile already records this convention for `check-release-consistency` (`Makefile:221-226`). **Recommendation:** add an optional alias
  ```make
  .PHONY: check-consumer-surface
  check-consumer-surface: ## Compile and run the consumer-surface muster (ADOPT-04); not part of check-gates (it builds)
  	@$(CARGO) test --test consumer_surface_muster
  ```
  and state in the PLAN that "wired into the existing gates" is satisfied by `make clean-code` + the CI jobs above; an extra CI step is optional (`cargo check --all-targets` already compiles it).

### Pattern 4: Proven red (answers orchestrator Q3) — see its own section below

### Recommended plan decomposition (Nyquist-friendly; ~5 small plans)

| Plan | Wave | Autonomous | Content |
|---|---|---|---|
| 49-01 | 1 | yes | **Muster (TDD order for a guard: write → green → prove red).** Write `tests/consumer_surface_muster.rs` from the verified draft; add `[[test]]`; `cargo fmt`; run muster + three guards + clippy. |
| 49-02 | 2 | yes | **Proven red.** Run the scripted mutations (Commissary bool; dropped facade re-export; a field pin), capture transcripts in `49-MUSTER-RED-EVIDENCE.md`, verify tree restored (`git status --short` clean). Optional `make check-consumer-surface`. |
| 49-03 | 2 | yes | **`MIGRATION.md` `###` consumer note + CHANGELOG entry**; run `make check-gates`, `make api-surface`, assert no §9.2 / allowlist diff. |
| 49-04 | 2 | yes (docs) | **Consumer spec + `49-RETURN-report.md` template** (Paladin-side facts filled, consumer fields `PENDING`). |
| 49-05 | 3 | **no (`checkpoint:human-action`)** | **Report intake.** Owner relays spec, consumer files the return report; verify against the intake checklist; only then mark ADOPT-01..03 `[x]`. |

### Anti-Patterns to Avoid
- **Freezing the whole API** (handoff D-4): pin only what the consumer imports.
- **Pinning a path that does not resolve to look complete:** `looks_like_auth_failure` is private. Pinning it would need `pub` — a Paladin API change, forbidden.
- **A "red proof" whose error is inside the library.** Re-adding a `bool` to `Commissary::new` without patching `from_port` and the RAG caller makes `paladin-llm`/`paladin-memory` fail first — a vacuous proof (see Pitfall 3).
- **Naming removed items in scanned files.** The muster must not contain `Node<`, `Node::new`, `battalion::ErrorStrategy`, `battalion::NodeError`, or a `battalion::{…}` import naming them (Pitfall 1). Describe removals in `MIGRATION.md` / `.planning/` only.
- **Executing a pin** (Pattern 2).
- **A shim, `#[deprecated]` alias, `from_total`, or `LlmResponse::new` added "to help the consumer"** — all forbidden (clean break; zero §9.2 rows).

## Proven Red (mechanical, reversible)

Measured on this tree with the draft muster temporarily at `tests/consumer_surface_muster.rs` (since removed; `git status --short` clean afterward). `cargo check --offline --test consumer_surface_muster` takes ~6–12 s warm.

**Experiment 1 — the handoff's own example (valid proof).** Add `_is_exact_counter: bool` after `counter` in `Commissary::new` **and** patch the only two non-test callers so the library still compiles: `from_port` (`crates/paladin-llm/src/services/commissary.rs:408`) and `Commissary::new("rag", …, plan)?` (`crates/paladin-memory/src/services/rag_retrieval_service.rs:244`). Only these two non-test call sites exist outside tests/examples [VERIFIED: grep]. Result, exit 101:
```
error[E0308]: mismatched types      --> tests/consumer_surface_muster.rs:71:48   (the fn-pointer pin)
error[E0061]: this function takes 5 arguments but 4 arguments were supplied
                                    --> tests/consumer_surface_muster.rs:84:13   (the call pin)
error: could not compile `paladin-ai` (test "consumer_surface_muster") due to 2 previous errors
```
The third `--> crates/paladin-llm/…:344:12` line is only a "function defined here" note. Restored by `git checkout -- <the two files>`.

**Experiment 3 — drop a facade re-export (valid proof).** Remove `ConsignmentItem` from the `pub use paladin_llm::services::commissary::{…}` group at `src/lib.rs:200`: `error[E0432]: unresolved import paladin::ConsignmentItem --> tests/consumer_surface_muster.rs:33:5`, exit 101, one error, library compiled.

**Experiment 2 — widen `TokenUsage::with_reasoning` to `u64` (INVALID proof; do not use).** The library itself stops compiling (`E0308` in `paladin-llm`'s adapters, which pass `u32` variables), so the muster is never reached. Recorded here as the cautionary example for the acceptance criterion.

**Acceptance criterion for the plan task** (mechanical, per experiment):
1. Baseline: `git status --short` shows only the intended working changes; muster green (`cargo test --offline --test consumer_surface_muster` → `1 passed`).
2. Apply the scripted mutation (perl/sed one-liners in a script with `trap 'git checkout -- <files>' EXIT`).
3. `cargo check --offline --test consumer_surface_muster` **exits non-zero**.
4. The output contains **exactly one** `error: could not compile` line and it names ``(test "consumer_surface_muster")`` — i.e. no library crate failed first; and ≥1 `--> tests/consumer_surface_muster.rs:` location.
5. After the trap restore, `git diff --stat -- crates src` is empty and the muster is green again.
6. Commit the transcript (`49-MUSTER-RED-EVIDENCE.md`); **do not** commit any mutated source.

Trait-implementer pins (E0046/E0053) cannot be red-proven cheaply (adding a required method breaks every in-tree implementer); they are covered by construction (the stubs implement only the required methods) and need no experiment. A scripted version of experiments 1 and 3 (`red_proof_commissary.sh`, `red_proof_misc.sh`) was run from the session scratchpad; the plan should recreate them as plan-local scripts, not commit them under `scripts/` (a `scripts/*.sh` addition would pull in shellcheck and the `test-shell-guards` convention).

## MIGRATION.md Consumer Note (answers orchestrator Q5)

**Where.** `MIGRATION.md` ends at §9.8 (headings `## 9.1` … `## 9.8`; no §9.9) [VERIFIED]. §9.8 is a numbered v0.9.0 → v0.10.0 *operator* checklist (steps 1–7). Recommend a **new `### Library consumers: v0.10.1 → v0.11.0` subsection appended to the end of §9.8** (matches the handoff's wording "§9.8 upgrade checklist gains any consumer-facing note" and ADOPT-05's text; leaves the program overview's 9.1–9.8 numbering intact). A new `## 9.9` is an acceptable alternative (Claude's discretion). Do **not** put it in §9.2.

**Guard compatibility.**
- `scripts/check-migration-allowlist.sh` reads only the span between `^## 9\.2 ` and `^## 9\.3 ` and only lines matching `^\| \``. A `###` after §9.8 is never read; adding no table row starting `| \`` inside §9.2 keeps the pair sets equal [VERIFIED: script source; `make check-gates` baseline green in 0.99 s on this tree].
- `tests/entity_rename_guard.rs` and `tests/legacy_removal_guard.rs` both skip root `MIGRATION.md` by name (their `HISTORY_FILES`) and every `CHANGELOG.md`, so the note and the CHANGELOG entry may name the removed items. They **do** scan `Cargo.toml`, workflow YAML, other `.md` files and `tests/*.rs`: keep the removed names out of the `[[test]]` comment block, any CI step text and the muster itself.
- `scripts/check-deprecations.sh` only concerns `#[deprecated]` items; none are added.

**Content (ordered).**
1. **Scope line:** applies to a library consumer (path/submodule dependency) moving from the `v0.10.1` tag to `v0.11.0`; the operator checklist above is separate.
2. **The D-2 pitfall (applies at `v0.10.1` already):** there is **no total-only `TokenUsage` constructor** (`TokenUsage::from_total` was deleted outright in Phase 31 — §9.2 `TokenUsage` row, D-05). `TokenUsage { total_tokens: n, ..Default::default() }` compiles but breaks the documented invariants (`total == prompt + completion`, `cache_read + cache_write <= prompt`, `reasoning <= completion`) **and is not additive**: `impl Add for TokenUsage` recomputes `total_tokens = prompt.saturating_add(completion)` (`token_usage.rs:131-149`), so a fallback-shaped value adds **0** to a `+`/`Sum`. Recommend: carry a full `TokenUsage` and accumulate per-call `LlmResponse.usage` with `+=` / `.sum::<TokenUsage>()` (both exist at `v0.10.1`, saturating); build literals with `..Default::default()` or `TokenUsage::new(p, c)` + `with_*`.
3. **Break list** — DR-1..DR-11 as a table (Item | What changed | Replacement | §9.2/§9.1 anchor), each row pointing at the existing register row; DR-6 and DR-7 are the two the ROADMAP did not name.
4. **Insulation advice (consumer-side, no Paladin change):** prefer `PaladinResult { .., ..Default::default() }` or `PaladinResult::new(..)` over a full literal (both exist at `v0.10.1`; `Default` is why `PaladinResult` is deliberately not `#[non_exhaustive]`, D-26); `LlmResponse` has no `Default` and no constructor (and must not gain one for this phase), so a single test helper `fn mock_response(usage) -> LlmResponse` reduces the 22 literal sites to one.
5. **Pointer to the muster:** "`tests/consumer_surface_muster.rs` is the executable form of this list; a red muster means the next bump needs a row here."

Also add a short `CHANGELOG.md` `[Unreleased]` entry (Added: the muster test and its `[[test]]` declaration; Documentation: the consumer note). `scripts/check-changelogs.sh` only checks that each publishable crate has a `CHANGELOG.md` — unaffected.

## Consumer-Side Deliverable (answers orchestrator Q6)

**Honest representation.** The handoff §7: "Do this in a web3sec checkout if you have one. If you don't, hand it back with this section as the spec. Do not commit to web3sec from a standalone Paladin checkout." §9.1 accepts "the Paladin-side commits plus confirmation that §7 is still accurate". No sibling web3sec checkout exists (`ls /home/user` = `paladin-dev-env` only). The plan therefore:

- ships `49-CONSUMER-ADOPTION-SPEC.md` (self-contained, below) and `49-RETURN-report.md` (template);
- leaves ADOPT-01..03 as **`Pending (consumer-side)`** in REQUIREMENTS/ROADMAP — never `[x]` from Paladin evidence — and writes "Paladin-side complete; consumer-side awaiting report" in the phase status;
- ends with an `autonomous: false` `checkpoint:human-action` plan (the owner relays the spec and files the report), then an intake checklist run against the filled report.

**Spec contents (handoff §7 made self-contained + corrections).**
- **Step 0 (new): inventory the real imports.** The handoff's Appendix A is as of `35fd839` and is wrong on at least one item. Run in the consumer:
  ```bash
  rg -U -N --no-heading -o --type rust 'use\s+(paladin|paladin_ports|paladin_memory)\b[^;]*;' crates | sort -u
  rg -N --no-heading -o --type rust '\b(paladin|paladin_ports|paladin_memory)::[A-Za-z_][A-Za-z0-9_:]*' crates | sort -u
  ```
  and return the output in the report: it freezes muster v1.1.
- **Step 1 — pin:** `git -C paladin fetch --tags && git -C paladin checkout --detach v0.10.1 && git -C paladin rev-parse HEAD` → **`f7dae2676580786e9541fefee8f787623b46c70f`**. Correction: `git rev-parse v0.10.1` returns the **tag object** `7593ab4dc1217ef23d6f8c14f8c8b817d42ec0ae` (annotated tag); verify the **commit** (`HEAD` after the detach, or `v0.10.1^{commit}`).
- **Step 2 — D-1:** record `4b7bed87` as superseded. Paladin-verified evidence for the consumer's commit message: 7 planning files, `git diff --quiet 4b7bed87 v0.10.1 -- <7 paths>` exit 0, `4b7bed87` not an ancestor of `v0.10.1`, branch already on `origin`.
- **Step 3 — B-1/B-2/B-3 from `49-REFERENCE-adoption.diff`** (15 files, +39/−34; 22 `TokenUsage` literals; mechanical and unreviewed), **implementing D-2 losslessly** (next section).
- **Step 4 — gates:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` (≥ 2,592 / 0 / 2); run from a **full checkout** (the `include_str!` fixtures/benchmarks caveat); commit pointer + adaptation + `Cargo.lock` together.
- **Step 5 — grep acceptance:** `grep -rn 'is_exact_counter\|token_count: report' crates/` empty; update the five prose mentions.
- **Pre-flight for the *next* bump (advisory):** a one-line grep list for DR-1..DR-11 (`ErrorStrategy|NodeError|with_retry_policy|with_error_strategy|with_timeout|RateLimitExceeded|PaladinError::LlmError|is_retryable|is_terminal|BattalionError::Timeout|entity::node|\.node\b`) so web3sec can size the v0.11 bump; and the helper-function recommendations from the note.
- **Persistence check (handoff B-2 caveat):** confirm web3sec never serializes `PaladinResult`; per §9.4 (D-25) an old-key JSON deserializes to `usage == TokenUsage::default()` with no mapping forward.

**D-2 lossless design, specified precisely against the Paladin tree.**
- `LlmResponse.usage: TokenUsage` carries all six fields per call; the Anthropic and DeepSeek adapters populate the three optionals where the provider reports them (`with_cache_read`, `with_reasoning` at `v0.10.1`: anthropic 2 call sites, deepseek 3) and leave `None` otherwise ("provider did not report").
- `TokenUsage` implements saturating `Add`, `AddAssign` and `Sum` **at `v0.10.1`** (`token_usage.rs:131/151/157`), merging each `Option<u32>` as `None+None=None`, `None+Some(x)=Some(x)`, `Some+Some=saturating_add`, and **recomputing `total_tokens` as `prompt + completion`** (it never sums the inputs' `total_tokens`). So: accumulate with `usage += response.usage.clone();` or `responses.iter().map(|r| r.usage.clone()).sum::<TokenUsage>()` — no new helper is needed.
- web3sec change set: `RoutineReport.token_count: u32` → `RoutineReport.usage: TokenUsage` (and the fuzz/deductive outcome accumulators that feed it); `phalanx_dispatch.rs`: `PaladinResult { usage: report.usage, ..Default::default() }`-style (or `PaladinResult::new(..)`); the unit test `assert_eq!(result.token_count, 4242)` becomes `assert_eq!(result.usage.total_tokens, 4242)` with the mock's usage built via `TokenUsage::new(p, c)` so the invariant holds (e.g. `new(0, 4242)`).
- **Fallback** (`TokenUsage { total_tokens: n, ..Default::default() }`, the trial's choice) is acceptable per D-2 only with a site comment and a tech-debt entry; the spec must state its two defects: invariant violation and **non-additivity** under `Add`/`Sum`. (`PhalanxExecutionService` itself sums `result.usage.total_tokens` directly at `phalanx_service.rs:286`, so the fallback would not zero out *there* — but Formation/Chain/any consumer `Sum` would.)

**Return-report template fields** (`49-RETURN-report.md`, handoff §9): (1) consumer commit SHA containing the bump to `f7dae267…` — `PENDING`; (2) final D-2 choice + changed web3sec types — `PENDING`; (3) gate output: fmt / clippy / check / test totals vs 2,592/0/2 — `PENDING`; (4) Paladin commits for ADOPT-04/05 and branch — **filled by Paladin**, plus "`v0.10.2` warranted? **No** — no library defect surfaced; zero Paladin library source, §9.2 or allowlist changes (`git diff` evidence)"; (5) divergences — **pre-filled by Paladin**: Appendix A `looks_like_auth_failure` is private; Appendix A "resolves on v0.10.1" is not the muster's baseline (the muster targets HEAD per the ROADMAP); the tag-object vs commit SHA nuance; the larger drift register; the `Ground truth` four-item list is a subset of DR-1..DR-11; plus an "inventory returned (Step 0)" field `PENDING`.

**Report-intake checklist** (mechanical, run against the filled report): SHA equals `f7dae2676580786e9541fefee8f787623b46c70f`; `passed ≥ 2592`, `failed == 0`, `ignored` recorded (baseline 2); fmt/clippy/check exit codes recorded; D-2 choice present and, if fallback, the site-comment path and tech-debt entry id; "no `unwrap`/`expect`/`panic!` added" evidence (consumer diff grep); stale-grep empty; Step-0 inventory attached.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Total-only `TokenUsage` | `from_total` / a shim constructor in Paladin | Consumer carries a full `TokenUsage`; accumulate with the existing `Add`/`AddAssign`/`Sum` | `from_total` was deleted deliberately (D-05); adding anything is a forbidden API change and a new §9.2 row |
| Compile-failure assertions | `trybuild` / a custom `rustc` driver / `compile_fail` doctests | A scripted mutation + `cargo check` exit/location assertion | Not in the dependency graph; `cargo deny` governs additions; a one-off proof is enough because the *muster itself* is the standing check |
| Facade breakage detection | A custom API diff tool | The muster (consumer-specific) + the existing `make api-surface` / `cargo semver-checks` (library-wide) | Different jobs: the muster is narrow and consumer-shaped; the others are broad and allowlist-gated |
| A repo scan for "consumer symbols" | A new std-only scanner like the three guards | Real `use` statements | A text scan cannot catch a signature change; the compiler can |
| Consumer inventory | Hand-reading web3sec | The two `rg` commands in the spec | Reproducible and returns evidence in the report |

**Key insight:** the compiler is the only tool that understands "this call still type-checks". Every pin should be an ordinary Rust item that stops compiling when the surface moves; everything else (scans, allowlists) is a different tool for a different failure.

## Runtime State Inventory

Phase 49 is an adoption/bump, not a rename. Paladin-side the change is a new test, a doc note and a CHANGELOG line; the "runtime state" questions concern the **consumer**.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | web3sec persists its own `RoutineReport`/registry types; trial found no `PaladinResult` serialization. §9.4 (D-25): an old `PaladinResult` JSON with `token_count` deserializes to `usage == TokenUsage::default()` | Consumer confirms in the report; no data migration unless it does persist `PaladinResult` |
| Live service config | None — Paladin runs no service for web3sec (it runs no server/engine binary) | None — verified by handoff §4 "Checked and not affecting web3sec" |
| OS-registered state | None | None |
| Secrets/env vars | None changed; the muster reads no env var (`DeepSeekConfig::from_env` is type-checked, never called) | None |
| Build artifacts | web3sec `Cargo.lock` changes with the bump; stale `paladin/target` artifacts from the old pin; Dockerfile comment "Rust pinned to match paladin's image" is stale (D-3, optional) | Commit `Cargo.lock` with the bump; clean build recommended; Dockerfile comment optional |

## Common Pitfalls

### Pitfall 1: The repo-scan guards read the muster
**What goes wrong:** `tests/entity_rename_guard.rs` scans every `.rs/.md/.toml/.yaml/.yml` file (skipping only `target`, `node_modules`, `.git`, `.planning`, `.project`, every `CHANGELOG.md`, root `MIGRATION.md`, the two `docs/src/api-reference` guides, the semver allowlist and itself) and fails on the old wrapper path, `<old name><`/`::new`/`::<` forms and the `…Version` family. `tests/legacy_removal_guard.rs` fails on `battalion::` + the removed type names, and on a `battalion::{…}` import naming them. A comment like "was `Node<T>`" in the muster or in `Cargo.toml` fails the build.
**How to avoid:** the muster imports only post-rename names and describes removals as "the old base wrapper" / "the removed battalion error strategy" — or not at all. Run the three guards in plan 49-01 (measured: pass with the draft present, 8 + 3 + 3 tests, ~11 s).
**Warning signs:** `entity_rename_guard` / `legacy_removal_guard` red after adding the file or a doc line.

### Pitfall 2: `clippy::type_complexity` on fn-pointer pins
**What goes wrong:** `make lint` / CI clippy (`-D warnings`, all targets) rejects the multi-argument `fn(...) -> Result<..> = Commissary::new` annotations [VERIFIED: 2 errors on the first draft].
**How to avoid:** file-level `#![allow(dead_code, clippy::type_complexity)]` with a one-line reason (the pins are deliberately spelled out). With it, `cargo clippy --test consumer_surface_muster -- -D warnings` is clean [VERIFIED].

### Pitfall 3: Vacuous red proof
**What goes wrong:** mutating a library signature so that the library fails first proves nothing about the muster (Experiment 2).
**How to avoid:** patch every non-test caller; require the single `could not compile` line to name the muster target.

### Pitfall 4: `impl Into<String>` and fn-pointer coercion
`Commissary::new` takes `provider: impl Into<String>`. The coercion `let _: fn(String, …) -> … = Commissary::new;` infers the anonymous parameter and compiles [VERIFIED]. Add a real call with a `&str` literal as well so narrowing the bound (e.g. to `String`) is caught.

### Pitfall 5: Silent skip when features are off
With `required-features`, `cargo test` under `--no-default-features` skips the target silently. The explicit form `cargo test --test consumer_surface_muster` errors loudly, so use that form in the optional Make alias and in any CI step; default CI jobs have the features on.

### Pitfall 6: Tag object vs commit
`git rev-parse v0.10.1` = `7593ab4d…` (annotated tag object); the commit is `f7dae2676580786e9541fefee8f787623b46c70f` (`git rev-parse v0.10.1^{commit}`). The handoff's "expect `f7dae267…`" is for `rev-parse HEAD` after the detach — correct, but a naive `rev-parse v0.10.1` check fails.

### Pitfall 7: Treating the handoff's Appendix A as authoritative
It is as of `35fd839`, lists a private function, names three removed items and a renamed one, and omits the types web3sec only reaches transitively (`BattalionResult`, `BattalionConfig` methods, `Phalanx::new`). The muster must be built from the consumer's real `use` lines (Step 0) — v1 from the re-verified Appendix A, v1.1 after the inventory arrives.

### Pitfall 8: Mistaking the muster's baseline
The handoff's criterion 4 ("passes on `v0.10.1`") is superseded by the ROADMAP ("as it exists on the current tree"). The muster is **expected to fail** against `v0.10.1` (`Entity`, `cost`, `PromptItem.entity` do not exist there); do not "fix" that.

### Pitfall 9: Phases 50 and 51 will turn the muster red on purpose
Phase 51 adds a reasoning-effort field to `PaladinData` and `LlmRequest`; Phase 50 touches the nine adapter/config pairs and `LlmRequest`. A strict `PaladinData` literal and the adapter-config pins will flip red, which is the muster doing its job. Those phases must update the pin and carry the matching `MIGRATION.md` consumer-note row in the same commit (ROADMAP Phase 50 already says "the muster then catches this phase's own" changes; Phase 47 requires the muster green before `v0.11.0` is tagged). Note this in the PLAN's follow-ups; it is not Phase 49 work.

### Pitfall 10: Sandbox-only test flake
`cargo test --workspace` in this environment has two known failures (`infrastructure::web::run_api_wiring::tests::…persists_run_traces_when_trace_persist_is_set` / `…no_run_traces_by_default`) that also fail on unmodified HEAD (Phase 48 research, Pitfall 10) — compare against that baseline.

## Code Examples

### The verified draft muster (compiled green + clippy-clean + guard-compatible on `0afec211`)

The 278-line rustfmt-clean source lived in the research session's scratchpad (ephemeral, not committed); the listing below is the same content with whitespace condensed for reading (run `cargo fmt --all` after pasting). It carries `#![allow(dead_code, clippy::type_complexity)]`. It is the executor's starting point; **the executor should add the deltas listed after it**.

```rust
//! Consumer-surface muster (ADOPT-04): every path and signature web3sec imports.
//! COMPILE-TIME ONLY. Nothing here is executed except one trivial `#[test]`.
#![allow(dead_code, clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

// ---- facade `paladin::` (features llm-anthropic + llm-deepseek) ----
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::arsenal::{Armament, ArmamentCall, ArmamentResult, ArsenalError};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin::infrastructure::adapters::arsenal::mcp_protocol::MCPClient;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{ArsenalPort, BattalionResult, LlmPort, PaladinError, PaladinResult, Phalanx, StopReason};
use paladin::{AnthropicAdapter, AnthropicConfig, Commissary, CommissaryError, CommissaryPlan, Consignment,
              ConsignmentItem, DeepSeekAdapter, DeepSeekConfig, MockLlmAdapter, MultiStepMockLlmPort};
// ---- direct path deps ----
use paladin_memory::token_counter::HeuristicTokenCounter;
use paladin_ports::output::llm_port::{FinishReason, LlmError, LlmRequest, LlmResponse,
                                      ProviderCapabilities, StreamingResponse, TokenUsage};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::token_counter_port::TokenCounterPort;

/// The facade prelude and the ports crate name ONE `LlmError`/`LlmRequest`/`LlmResponse`.
fn prelude_and_ports_agree(e: paladin::prelude::LlmError, q: paladin::prelude::LlmRequest,
                           r: paladin::prelude::LlmResponse) -> (LlmError, LlmRequest, LlmResponse) { (e, q, r) }

fn usage_literal() -> TokenUsage {   // B-3: six fields, no `..`
    TokenUsage { prompt_tokens: 1, completion_tokens: 2, total_tokens: 3,
                 cache_read_tokens: None, cache_write_tokens: None, reasoning_tokens: None }
}

fn commissary_signatures() {         // B-1: the fn-pointer pins
    let _new: fn(String, ProviderCapabilities, Arc<dyn TokenCounterPort>, CommissaryPlan)
        -> Result<Commissary, CommissaryError> = Commissary::new;
    let _from_port: fn(&dyn LlmPort, Arc<dyn TokenCounterPort>, CommissaryPlan)
        -> Result<Commissary, CommissaryError> = Commissary::from_port;
    let _tokens: fn(&Commissary) -> u32 = Commissary::allotted_tokens;
    let counter: Arc<dyn TokenCounterPort> = Arc::new(HeuristicTokenCounter);
    let plan = CommissaryPlan { reserved_completion_tokens: 1, model_hint: "m".to_string(), ..Default::default() };
    let _ = Commissary::new("literal-str", ProviderCapabilities::default(), counter, plan);
}

fn usage_signatures() {              // D-2 surface: constructors, builders, Add/AddAssign/Sum
    let _new: fn(u32, u32) -> TokenUsage = TokenUsage::new;
    let _a: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_cache_read;
    let _b: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_cache_write;
    let _c: fn(TokenUsage, u32) -> TokenUsage = TokenUsage::with_reasoning;
    let _sum: TokenUsage = vec![TokenUsage::default()].into_iter().sum();
    let mut acc = TokenUsage::default();
    acc += TokenUsage::new(1, 2);
    let _total = acc + usage_literal();
    let _lossless = TokenUsage { prompt_tokens: 1, ..Default::default() };
}

fn result_literals() -> (PaladinResult, LlmResponse) {   // B-2, DR-5, DR-6: full literals, no `..`
    let p = PaladinResult { output: String::new(), usage: usage_literal(), cost: None,
        execution_time_ms: 0, loop_count: 1, stop_reason: StopReason::Completed,
        plan: None, handoff_history: Vec::new(), served_by: None };
    let l = LlmResponse { id: Uuid::new_v4(), request_id: Uuid::new_v4(), model: String::new(),
        content: String::new(), finish_reason: FinishReason::Stop, usage: usage_literal(), cost: None,
        created_at: Utc::now(), metadata: HashMap::new(), function_call: None };
    (p, l)
}

fn paladin_literal() -> Paladin {    // DR-3: Paladin = Entity<PaladinData>; 13 payload fields
    let data = PaladinData { system_prompt: String::new(), name: String::new(), user_name: String::new(),
        model: String::new(), temperature: 0.0, max_loops: MaxLoops::Fixed(1), stop_words: Vec::new(),
        status: PaladinStatus::Idle, vision_enabled: false, autonomous_planning: false,
        autonomous_prompts: false, agent_description: String::new(), dynamic_temperature: false };
    Entity::new(data, Some("n".to_string()))
}

fn prompt_signatures() {             // DR-4: `.entity.data`
    let item = PromptItem::new(PromptType::User(UserPrompt { query: String::new(), context: None }));
    let _req = item.map(|p| { let _read: &PromptType = &p.entity.data.prompt_type; LlmRequest::new("m", p) });
}

// --- minimal implementers: ONLY the required methods, so a new required method or a changed
//     signature is a hard error ---
struct MusterLlm;
#[async_trait]
impl LlmPort for MusterLlm {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> { Ok(result_literals().1) }
    async fn generate_stream(&self, _request: LlmRequest)
        -> Result<Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
    { Err(LlmError::NetworkError(String::new())) }
    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> { Err(LlmError::Timeout(String::new())) }
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> { Ok(Vec::new()) }
    fn get_provider_name(&self) -> &'static str { "muster" }
    fn get_capabilities(&self) -> ProviderCapabilities { ProviderCapabilities::default() }
}
struct MusterPaladinPort;
#[async_trait]
impl PaladinPort for MusterPaladinPort {
    async fn execute(&self, paladin: &Paladin, _input: &str) -> Result<PaladinResult, PaladinError> {
        let _model: &str = &paladin.data.model; Ok(result_literals().0) }
    async fn execute_stream(&self, _p: &Paladin, _i: &str) -> Result<PaladinStream, PaladinError> {
        Err(PaladinError::ExecutionError(String::new())) }
    fn validate(&self, _p: &Paladin) -> Result<(), PaladinError> { Ok(()) }
}
struct MusterArsenal;
#[async_trait]
impl ArsenalPort for MusterArsenal {
    async fn list_armaments(&self) -> Vec<Armament> { Vec::new() }
    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        Ok(ArmamentResult::success(call.call_id, serde_json::Value::Null, 0)) }
    fn validate_call(&self, _c: &ArmamentCall) -> Result<(), ArsenalError> { Ok(()) }
}

async fn service_and_adapter_signatures(args: &[String]) {   // NEVER CALLED
    let llm: Arc<dyn LlmPort> = Arc::new(MusterLlm);
    let breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    let arsenal: Option<Arc<dyn ArsenalPort>> = Some(Arc::new(MusterArsenal));
    let service = PaladinExecutionService::new(llm, breaker.clone(), None, arsenal);
    let _r: Result<PaladinResult, PaladinError> = service.execute(&paladin_literal(), "x").await;
    let _cb: Result<u32, PaladinError> = breaker.call_async(async { Ok(1u32) }).await;
    let port: Arc<dyn PaladinPort> = Arc::new(MusterPaladinPort);
    let phalanx_service = PhalanxExecutionService::new(port);
    let phalanx: Result<Phalanx, BattalionError> = Phalanx::new(vec![paladin_literal()], BattalionConfig::new("n"));
    if let Ok(phalanx) = phalanx {
        let _b: Result<BattalionResult, BattalionError> = phalanx_service.execute(&phalanx, "x").await; }
    let _c: Result<MCPClient, ArsenalError> = MCPClient::connect_stdio("cmd", args).await;
    let _adapters = (
        AnthropicAdapter::new(AnthropicConfig::new(String::new(), String::new(), String::new(), 1)),
        DeepSeekConfig::from_env().map(DeepSeekAdapter::new),
        MockLlmAdapter::new(),
        MultiStepMockLlmPort::new(Vec::new()),
    );
    let _items = (Consignment::new(), ConsignmentItem { label: String::new(), body: String::new(), priority: 0 });
}

#[test]
fn the_consumer_surface_still_resolves() {
    // Reaching this point means every `use`, fn-pointer, literal and trait-impl pin type-checked.
}
```

**Deltas the executor should add to the draft (not yet compiled):** (a) a `PaladinResult { output, usage, execution_time_ms, loop_count, stop_reason, ..Default::default() }` literal (the consumer-preferred insulated form; `Default` exists at `v0.10.1` and HEAD); (b) `PaladinResult::new(String, TokenUsage, u64, u32, StopReason)` fn-pointer pin; (c) `BattalionConfig::with_description`, `Phalanx::with_max_concurrency`, `PromptItem::new` pins only if the Step-0 inventory shows them (do not guess); (d) a header comment stating the strictness policy, the "never call a pin" rule, the baseline (HEAD, **not** `v0.10.1`) and the "update the pin and the §9.2 row together" rule — without any removed identifier spelled out (Pitfall 1); (e) `cargo fmt --all` (the draft here is already rustfmt-clean).

### Accumulating usage losslessly (consumer-side D-2)
```rust
// Source: crates/paladin-core/src/platform/container/token_usage.rs (Add / AddAssign / Sum, present at v0.10.1)
let mut usage = TokenUsage::default();
for response in &responses {
    usage += response.usage.clone();       // saturating; total recomputed as prompt + completion
}
let total: TokenUsage = responses.iter().map(|r| r.usage.clone()).sum();
```

### Scripted proven-red mutation (shape; recreate as a plan-local script)
```bash
set -uo pipefail; cd "$REPO"
FILES=(crates/paladin-llm/src/services/commissary.rs crates/paladin-memory/src/services/rag_retrieval_service.rs)
trap 'git checkout -- "${FILES[@]}"' EXIT
perl -0pi -e 's/(counter: Arc<dyn TokenCounterPort>,\n)(        config: CommissaryPlan,\n    \) -> Result<Self, CommissaryError> \{\n        let provider = provider\.into\(\);)/$1        _is_exact_counter: bool,\n$2/' "${FILES[0]}"
perl -0pi -e 's/(llm\.get_capabilities\(\),\n            counter,\n)(            config,)/$1            false,\n$2/' "${FILES[0]}"
perl -0pi -e 's/Arc::clone\(&self\.token_counter\), plan\)\?;/Arc::clone(&self.token_counter), false, plan)?;/' "${FILES[1]}"
cargo check --offline --test consumer_surface_muster >"$LOG" 2>&1; rc=$?
[ "$rc" -ne 0 ] && [ "$(grep -c '^error: could not compile' "$LOG")" -eq 1 ] \
  && grep -q 'could not compile `paladin-ai` (test "consumer_surface_muster")' "$LOG" \
  && grep -q -- '--> tests/consumer_surface_muster.rs:' "$LOG" && echo RED-PROVEN
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Commissary::new(.., is_exact_counter: bool, ..)` | exactness read live from `TokenCounterPort::is_exact()` | v0.10.0 (32-01) | B-1: delete the argument |
| `PaladinResult.token_count: u32` | `usage: TokenUsage` | v0.10.0 (31) | B-2 |
| `TokenUsage` 3 fields, `from_total` | 6 fields, saturating `Add`/`AddAssign`/`Sum`, no `from_total` | v0.10.0 (31) | B-3, D-2 |
| `Node<T>` base wrapper, `.node` | `Entity<T>`, `.data` | Phase 48 (v0.11) | DR-3/DR-4 |
| Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError`/timeouts | one Aegis policy + structured `NodeError` | Phase 44 (v0.11) | DR-1/2/8/9/10 |
| `LlmResponse`/`PaladinResult` without cost | `cost: Option<Cost>` | Phase 38 (v0.11) | DR-5/DR-6 |

**Deprecated/outdated:** the handoff's Appendix A (as of `35fd839`); `.github/copilot-instructions.md` still shows an illustrative `PaladinError::LlmError(String)` enum that no longer exists (an illustration, not source of truth).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo llvm-cov` excludes `tests/` from the report by default (README, fetched; seam tier LOW for `webfetch`; tool not installed locally so not re-measured) | Validation / coverage | Negligible: even if counted, ~300 never-executed lines against ~54k (85.01 % measured per `scripts/coverage.sh`) cannot take the floor below 82 % |
| A2 | web3sec constructs `PaladinResult`/`LlmResponse` by full literal and has no other use of the DR-7..DR-10 items (handoff §4 says "full struct literal", "no exhaustive match"; it says nothing about constructing `PaladinError::LlmError`, `LlmError::RateLimitExceeded`, or setting `BattalionConfig` fields) | Drift register | The DR-7..DR-10 rows are still correct as *Paladin facts*; only their relevance to web3sec is unknown until the Step-0 inventory returns |
| A3 | The muster should pin `BattalionConfig::new`, `Phalanx::new`, `PhalanxExecutionService::{new, execute}`, `PaladinExecutionService::{new, execute}`, `CircuitBreaker::{new, call_async}`, `MCPClient::connect_stdio` and `AnthropicConfig::new` — chosen as the *likeliest* uses of imported types, not from consumer source | Muster | A real use the muster does not pin is a false-green; mitigated by Step 0 and muster v1.1 |
| A4 | web3sec's `Cargo.toml` keeps `paladin-ai`'s default features on (handoff shows `paladin-ai = { path = "paladin" }`) | Imported surface | If it used `default-features = false` only the two named LLM features matter — the muster's `required-features` already equals that set |
| A5 | The owner prefers keeping ADOPT-01..03 `Pending` until the consumer's return report exists (rather than sealing the phase on Paladin-side evidence alone) | Consumer-side deliverable | If the owner wants the phase sealed earlier, ROADMAP success criteria 1–3 would need re-wording by the owner — not a Phase 49 plan decision |

## Open Questions

1. **What does web3sec actually import and construct?**
   - What we know: Appendix A (as of `35fd839`) plus the trial diff's 15 touched files; one Appendix-A item is provably impossible (`looks_like_auth_failure`).
   - What's unclear: use of DR-7..DR-10 items, `BattalionConfig` builder calls, `PaladinData` construction style (literal vs `Default`), `MCPClient` method set.
   - Recommendation: ship the draft as muster v1; make the Step-0 `rg` inventory a required field of the return report; schedule a cheap muster v1.1 patch when it arrives (tracked, not blocking the phase's Paladin-side plans).
2. **Seal the phase before the consumer report arrives?**
   - What we know: ADOPT-01..03 can only be closed by the §9 report; the ROADMAP criteria 1–3 demand consumer facts.
   - Recommendation: seal Paladin-side plans, keep ADOPT-01..03 `Pending`, record "awaiting consumer report" in STATE; ask the owner (do not decide in the plan).
3. **`## 9.9` vs `###` under §9.8** — Claude's discretion; recommendation above (`###` under §9.8).
4. **Add the optional `make check-consumer-surface` alias?** — Recommendation: yes (3 lines, discoverability), not in `check-gates`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test/clippy/fmt | yes | cargo/rustc 1.97.1 (repo pin) | — |
| `cargo clippy`, `rustfmt` | lint/format | yes | with 1.97.1 | — |
| `make` | gates | yes | GNU Make 4.3 | run the underlying commands |
| `cargo-public-api` + `nightly-2026-09-20` | `make api-surface` | yes | 0.52.0; toolchain `nightly-2026-09-20` installed | — |
| `cargo-semver-checks` | semver job (not run by this phase) | yes | 0.50.0 | CI-only |
| `cargo-deny`, `cargo-audit` | `make security` | yes | 0.20.2 / 0.22.2 | — |
| `cargo-llvm-cov` | `make coverage` | **no** | — | coverage is a CI-only measurement here; reasoning in Validation (A1) |
| Redis / RustFS services | full integration + coverage | not probed (Docker CLI present, daemon not exercised) | Docker 29.8.2 | CI; Phase-48-known 2 sandbox flakes |
| `shellcheck` | `make lint-shell` (only if a script is added) | **no** | — | add no shell script under `scripts/` (recommended anyway) |
| web3sec repository | ADOPT-01..03 | **no** (not reachable) | — | spec + return-report template |
| Disk | builds | tight: 2.4 GB free of 252 GB at research end | — | do not build a second target tree (e.g., a `v0.10.1` worktree check) without freeing space |

**Missing dependencies with no fallback:** none for the Paladin half. The consumer half is externally blocked by design.
**Missing dependencies with fallback:** `cargo-llvm-cov`, `shellcheck`, services — CI covers them.

Baselines measured this session on `0afec211`: `make check-gates` exit 0 in 0.99 s; muster draft `cargo check` 12.6 s warm / `cargo test --test consumer_surface_muster` 44 s warm (1 passed); three guards 11 s (8 + 3 + 3 passed).

## Validation Architecture

> `.planning/config.json` has no `workflow.nyquist_validation` key (it holds only `workflow._auto_chain_active`, `worktree_skip_hooks`, `test_gate_timeout`), so validation is enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (libtest) for Rust; shell-script gates (`scripts/check-*.sh`) |
| Config file | `Cargo.toml` (`[[test]]` tables); `Makefile`; `.github/workflows/ci.yml` |
| Quick run command | `cargo test --offline --test consumer_surface_muster` (~45 s warm; ~12 s for `cargo check --test …`) |
| Full suite command | `cargo test --workspace --features integration-tests -- --test-threads=1` (CI job; needs Redis/RustFS; 2 known sandbox flakes) |
| Phase gate commands | `cargo fmt --all -- --check`, `make lint`, `make check`, `make check-gates`, `make api-surface`, `make doc-check`; coverage via CI's `coverage` job |
| Estimated runtime | quick ≈ 45 s; gates ≈ 1 s + `api-surface` (minutes, nightly) + clippy all-features (minutes warm) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ADOPT-01 | Consumer pinned at `f7dae267…`, `4b7bed87` superseded | **consumer-side — verifiable only via the return report** | Paladin-side precheck: `git rev-parse v0.10.1^{commit}` = `f7dae2676580786e9541fefee8f787623b46c70f`; `git diff --quiet 4b7bed87 v0.10.1 -- $(git show --name-only --format= 4b7bed87)` exits 0 | template ❌ Wave 0 |
| ADOPT-02 | B-1/B-2/B-3 adapted; D-2 decided | **consumer-side — return report** | intake checklist (D-2 choice present; tech-debt id if fallback; stale-grep empty) | ❌ Wave 0 |
| ADOPT-03 | Consumer gates green, ≥ 2,592 / 0 / 2 | **consumer-side — return report** | intake checklist (totals + exit codes) | ❌ Wave 0 |
| ADOPT-04 | Muster compiles and passes | compile-only integration test | `cargo test --offline --test consumer_surface_muster` | ❌ Wave 0 |
| ADOPT-04 | Muster is proven red | scripted mutation (3 experiments) | plan-local script; criteria 1–6 in "Proven Red" | ❌ Wave 0 |
| ADOPT-04 | Muster wired into gates | gate runs | `make check` and `make lint` include it; `cargo test --test consumer_surface_muster --no-default-features` errors loudly | existing gates ✅ |
| ADOPT-04 | Guards still green with the new file | repo-scan guards | `cargo test --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard` | ✅ |
| ADOPT-05 | MIGRATION consumer note present and correct | grep acceptance | `grep -n 'no total-only' MIGRATION.md`; `grep -c` of DR-1..DR-11 names in the new subsection; note sits after `## 9.8` and before EOF | ❌ Wave 0 |
| ADOPT-05 | Zero new §9.2 rows, zero allowlist change | diff assertion | `git diff 0afec211 -- .cargo/semver-checks-allowlist.toml` empty; `git diff 0afec211 -- MIGRATION.md \| grep -E '^[+-]\| \`'` empty; `make check-gates` | ✅ |
| ADOPT-05 | No Paladin library change/shim/deprecation | diff assertion | `git diff --name-only 0afec211 -- crates src` empty (plus `Cargo.toml` diff limited to the one `[[test]]` table); `make api-surface` exits 0 | ✅ |
| (cross) | 82 % floor | CI | `coverage` job (`scripts/coverage.sh`) | ✅ (CI only; tool absent locally) |

### Sampling Rate
- **Per task commit:** `cargo test --offline --test consumer_surface_muster` (plan 49-01/02) or `make check-gates` (plan 49-03).
- **Per wave merge:** the three guard tests + `cargo clippy --test consumer_surface_muster -- -D warnings` + `cargo fmt --all -- --check` + `make check-gates`.
- **Phase gate:** `make clean-code`, `make check-gates`, `make api-surface`, the diff assertions above, and CI's full suite and coverage green before `/gsd-verify-work`; ADOPT-01..03 additionally require the filed return report (intake checklist).

### Wave 0 Gaps
- [ ] `tests/consumer_surface_muster.rs` — covers ADOPT-04
- [ ] `Cargo.toml` `[[test]] consumer_surface_muster` (`required-features`) — ADOPT-04 wiring
- [ ] `.planning/phases/49-…/49-MUSTER-RED-EVIDENCE.md` — ADOPT-04 red proof record
- [ ] `.planning/phases/49-…/49-CONSUMER-ADOPTION-SPEC.md`, `49-RETURN-report.md` — ADOPT-01..03 (Paladin side)
- [ ] `MIGRATION.md` `###` consumer note + `CHANGELOG.md` entry — ADOPT-05
- Framework install: none — existing test infrastructure covers all Paladin-side requirements.

## Security Domain

`security_enforcement` is not set in `.planning/config.json` (absent = enabled).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth code touched) |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | no | No external input; the muster is compile-time only |
| V6 Cryptography | no | — |
| V10 Malicious Code / V14 Config (supply chain, secrets) | yes (light) | No new dependency (`cargo deny` / `make security` unchanged); no credential in the file (`AnthropicConfig::new(String::new(), …)`; `gitleaks` pre-commit); `DeepSeekConfig::from_env` is type-checked only |

### Known Threat Patterns for a compile-only Rust test

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| An executed pin spawns a process (`MCPClient::connect_stdio`) or reads the environment (`DeepSeekConfig::from_env`) | Elevation of privilege / Information disclosure | Pins live only in never-called functions; the single `#[test]` body is empty; review that no pin gained a call site |
| A real API key pasted into a literal while writing a config pin | Information disclosure | Use `String::new()`; `gitleaks` hook; manual credential-handling review (imported `security.instructions.md`) |
| A new dev/test dependency slips in | Tampering (supply chain) | None added; `[dependencies]` already provide every crate used; `make security` stays green |
| Consumer spec instructs a `git submodule`/network step | Tampering | Spec pins by commit SHA and verifies `rev-parse` before building |

## Sources

### Primary (HIGH confidence)
- This checkout at `0afec211`: `Cargo.toml`, `src/lib.rs`, `src/prelude.rs`, `src/core/**`, `src/application/services/**`, `src/infrastructure/**`, `crates/paladin-{core,ports,llm,memory,battalion}/src/**` — every path and signature in the surface tables was read or grepped.
- `git diff v0.10.1..HEAD` over the Appendix-A files; `git show v0.10.1:…` / `35fd839:…` for `looks_like_auth_failure`, `BattalionConfig`, `TokenUsage` (Add/AddAssign/Sum present, no `from_total`).
- `MIGRATION.md` §9.2 rows (`Node`, `ErrorStrategy`, `NodeError`, `BattalionResult`, `BattalionConfig`, `BattalionError`, `PaladinError`, `PromptItem`, `ContentItem`, `PaladinResult`, `LlmResponse`, `LlmError`, `TokenUsage`, `Commissary`, `StopReason`), §9.4, §9.8.
- `scripts/check-migration-allowlist.sh`, `scripts/check-changelogs.sh`, `Makefile`, `.github/workflows/ci.yml`, `scripts/coverage.sh`, `tests/{entity_rename,legacy_removal,treasurer_vocabulary}_guard.rs`.
- Measured by running: draft muster compile/clippy/fmt/test; three mutation experiments; three guard tests; `make check-gates`; `cargo test --no-default-features --test …` error text; `cargo metadata` target listing; `git diff --quiet 4b7bed87 v0.10.1 -- <paths>`.
- `49-HANDOFF-web3sec.md`, `49-REFERENCE-adoption.diff`, ROADMAP Phase 49, REQUIREMENTS ADOPT block, Phase 48 CONTEXT/RESEARCH.

### Secondary (MEDIUM confidence)
- cargo-llvm-cov README (https://github.com/taiki-e/cargo-llvm-cov) — default exclusion of `tests/`, `examples/`, `benches/`; fetched via `webfetch`, which the `classify-confidence` seam rates LOW (Assumption A1).

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; every dependency read from `Cargo.toml` and exercised by a compile.
- Architecture (muster mechanics, gate wiring, proven red): HIGH — compiled, linted, run and mutated on this tree.
- Surface re-verification and drift register: HIGH for Paladin facts (grep + diff + §9.2); the *relevance to web3sec* is A2/A3 (MEDIUM) until the Step-0 inventory returns.
- Consumer-side findings: N/A by construction (unreachable).
- Pitfalls: HIGH — each one was hit or measured.

**Research date:** 2026-10-14
**Valid until:** until the next surface-changing commit lands on this branch (Phases 46, 47, 50, 51 all do); re-run the draft compile and the three mutation experiments at plan-execution time. Otherwise 30 days.
