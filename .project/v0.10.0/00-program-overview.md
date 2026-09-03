# Paladin vNext — Program Overview & Specification Index

**Audience:** The implementation agent responsible for organizing, managing, and coding this program of work, and the verification agent that will audit completion.
**Status:** Approved for implementation planning.
**Baseline:** Paladin workspace **v0.9.0** (crates: `paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-content`, `paladin-notifications`, `paladin-web`, facade crate `paladin-ai`).
**Target release:** **v0.10.0** for the whole program (all workspace crates bump together, as today). See X-10 for the versioning rules and §9 for the mandatory `MIGRATION.md`.

---

## 1. Program Goal

Evolve Paladin from a pattern-oriented orchestration framework into a **durable agent execution runtime**. After this program, any multi-agent workflow run in Paladin must be:

1. **Stateful** — agents communicate through a typed, shared, mergeable state object, not bare strings.
2. **Durable** — execution state is checkpointed automatically at every step; a crashed, evicted, or restarted process resumes exactly where it stopped.
3. **Interruptible** — a workflow can pause at any node, wait indefinitely (seconds to days) for human input without holding compute, and resume.
4. **Inspectable** — the full checkpoint history of any run can be listed, replayed, and forked.
5. **Dynamic** — control flow (which node runs next, how many parallel workers spawn) can be decided at runtime by node output, not only at graph-build time.
6. **Fault-tolerant per node** — retries, timeouts, and typed error handlers are configurable on individual nodes, enabling compensation workflows.
7. **Operable** — runs execute in the background on a task queue, are addressable over an HTTP API, and emit a standardized trace stream.

## 2. Document Index

| Doc | Title | Epic ID prefix | Depends on |
|---|---|---|---|
| 01 | Battlefield State & Superstep Execution Engine | `ENG` | — (keystone) |
| 02 | Control Flow: Dynamic Routing, Fan-Out, Subgraphs, Custom Conditions | `CF` | 01 |
| 03 | Pause/Resume, Execution History, Graceful Shutdown | `HITL` | 01, 02 |
| 04 | Node-Level Fault Tolerance | `FT` | 01 (partial standalone) |
| 05 | Agent Runtime Enhancements (Middleware, Context, Store, Structured Output, Providers) | `RT` | mostly standalone |
| 06 | Platform API (Background Runs, Threads, Assistants, Schedules, Webhooks) | `PLAT` | 01, 03 |
| 07 | Observability, Trace Export, Visualization, Eval Harness | `OBS` | 01 (trace hooks) |

**Implementation order:** 01 → 02 → 03 and 04 (parallel) → 05 (parallelizable with 03/04) → 06 → 07. Items marked "standalone" in each doc may be scheduled earlier at the implementer's discretion.

## 3. Non-Negotiable Cross-Cutting Requirements

These apply to every requirement in every document. The verification pass will check them per-epic.

**X-01 (Architecture).** Hexagonal dependency rule is preserved: `paladin-core` depends on nothing internal; `paladin-ports` depends only on `paladin-core`; infrastructure adapters depend on core + ports; the facade assembles. No new port trait may import an SDK, database driver, or HTTP client. Any requirement below that names a trait states its home crate; if a requirement forces a violation, stop and flag it rather than violating the rule.

**X-02 (TDD).** Red-Green-Refactor. Every functional requirement (FR) below is written to be testable; each FR must be covered by at least one unit test, and each epic ships the integration tests named in its Test Plan. Workspace line coverage must remain ≥ 82% (per ADR-0006). All new public APIs need doc tests.

**X-03 (Backward compatibility).** Existing public APIs (`PaladinPort::execute(&Paladin, &str)`, existing Battalion services, existing web endpoints) must continue to compile and behave identically. New capabilities are additive. Where a legacy path is wrapped (e.g., string input → typed state), the wrapping must be lossless and documented. Deprecations are allowed with `#[deprecated]` but removals are not. The single sanctioned behavioral change is BUG-01 (§7); it MUST be listed in `MIGRATION.md` (§9). Any other behavioral change discovered during implementation is a stop-and-flag event, not a judgment call.

**X-04 (Serialization).** Every new persisted type carries a `schema_version: String` field and derives `Serialize`/`Deserialize` via serde. Loading a state with an unknown newer schema version is an explicit typed error, never a panic or silent misparse.

**X-05 (Concurrency).** All new port traits are `Send + Sync`. All shared mutable state uses `tokio::sync` primitives. Every new concurrency-sensitive component ships at least one `#[tokio::test(flavor = "multi_thread")]` stress test with exact-count assertions and an explicit timeout guard (follow the house pattern established in `src/application/services/orchestration/listener.rs`).

**X-06 (Errors).** No new stringly-typed error variants. New failure modes get `thiserror` enums with structured fields. Existing `String`-payload variants may remain but must not gain new call sites.

**X-07 (Feature flags).** New infrastructure adapters are gated behind cargo features on the umbrella crate, following the existing convention (`redis-queue`, `qdrant`, etc.).

**X-08 (Docs).** Each epic updates the mdBook docs and crate-level rustdoc. `cargo doc` must build without new broken intra-doc links.

**X-09 (Config).** Every new runtime behavior that is tunable gets a config struct in `src/config/` with `Default`, `validate()`, and `EnvOverridable` implementations, mirroring `CitadelConfig`.

**X-10 (Semver hygiene — Rust API surface).** "Additive" in X-03 is a semantic promise, but in Rust several additive-looking edits are compile-breaking for downstream crates: adding a variant to a public `enum` breaks exhaustive `match` arms; adding a public field to a `struct` breaks struct-literal construction and destructuring; adding a required method to a public trait breaks every implementor. The program touches existing public types in exactly these ways (see the register in §9.2). Rules:

1. **Inventory first.** Before modifying any *pre-existing* public type in any crate, the implementer records it in the `MIGRATION.md` register (§9.2): type, crate, kind of change, mitigation chosen. No exceptions — the verification pass diffs the public API and every unregistered change is a finding.
2. **Enums.** Every pre-existing public enum that gains a variant MUST be marked `#[non_exhaustive]` in the same change, **unless** doing so is itself unreasonable for the type's role (e.g., callers legitimately need exhaustive matching on `StopReason`). In that case the addition is recorded as a *deliberate minor-version-breaking* change in the register, and the type gets a doc comment listing the variants added in this release. Prefer `#[non_exhaustive]`; justify the exception in writing.
3. **Structs.** Every pre-existing public struct that gains a field MUST either (a) already be, or become, `#[non_exhaustive]`, with a builder or `Default` construction path guaranteed to exist and be doc-tested; or (b) be recorded as deliberate-breaking with justification. New fields on serde-persisted structs MUST carry `#[serde(default)]` so previously written data still deserializes.
4. **Traits.** A pre-existing public trait MUST NOT gain a required method. New trait methods MUST have default implementations (and the default must be *correct*, not `unimplemented!()`), or the capability goes on a new trait — which is why this program introduces `StructuredExecutorPort`, `WaypointPort`, `VaultPort`, etc. rather than extending `PaladinPort`/`CitadelPort`.
5. **Automated gate.** CI gains a `semver` job running `cargo semver-checks` against the v0.9.0 published crates for every publishable crate. The job MUST pass, with the only permitted exceptions being the deliberate-breaking entries in the register, each suppressed by an explicit, per-item allowlist checked into the repo (not a blanket ignore).
6. **Version bump.** All workspace crates release together as **v0.10.0**. Under semver's 0.x rules a minor bump *permits* breaking changes; this program uses that permission **only** for the register's deliberate-breaking entries and BUG-01. Nothing else may rely on it.
7. **Feature flags are not a loophole.** Default-off features must not change behavior or public types when off; enabling a feature may add items but must never remove or alter existing ones.

**X-11 (MSRV & dependency discipline).** The workspace MSRV is **Rust 1.85 (edition 2024)** and is advertised in the README badge and `rust-version` fields. This program adds dependencies with a real chance of raising it (JSON-schema generation, Postgres support in the SQL stack, the OpenTelemetry family, cron parsing, HMAC). Rules:

1. Every new dependency is added with an explicit version and checked with `cargo msrv verify` (or an equivalent CI job pinned to the MSRV toolchain building the full workspace with `--all-features`). The CI job is added in the first epic that introduces a new dependency and runs thereafter.
2. If a needed dependency cannot be satisfied at MSRV 1.85: first try an older compatible version; if none, the MSRV bump is proposed as a **stop-and-flag** item with the alternatives considered, recorded in `MIGRATION.md` §9.3, and applied in one place (`workspace.package.rust-version`) with the README badge updated in the same commit. An MSRV bump is a documented minor-version change, not a silent one.
3. `cargo deny` (licenses/bans/advisories) and `cargo audit` remain gating; new deps must clear both, and any new license category must be explicitly allowed in `deny.toml` with a note.
4. Feature-gated heavy dependencies (Postgres driver, OpenTelemetry exporters, embedding clients) MUST NOT be pulled into the default feature set of `paladin-ai`, so a default `cargo build` cost and dependency graph does not grow materially (verify with `cargo tree -e features` in the PR).

## 4. Ubiquitous Language Additions

New domain terms introduced by this program (use these names consistently in code and docs):

| Term | Meaning |
|---|---|
| **Battlefield** | The typed, shared state object for one workflow run. Nodes read it and emit deltas. (Doc 01) |
| **Dispatch** | A per-field merge function on the Battlefield that combines concurrent deltas deterministically. Called a *dispatch rule* or *reducer* interchangeably. (Doc 01) |
| **Superstep** | One engine iteration: select ready nodes → execute them (possibly in parallel) → merge deltas → checkpoint. (Doc 01) |
| **Waypoint** | One durable checkpoint of a run: Battlefield snapshot + frontier + metadata, addressed by `(thread_id, waypoint_id)`. (Doc 01) |
| **Thread** | A logical run/conversation identifier under which all Waypoints of a workflow execution are stored. (Doc 01) |
| **Directive** | A value a node may return to steer execution: state delta plus optional routing (`Goto`, `End`, `Halt`), replacing purely static edges. (Doc 02) |
| **Muster** | Dynamic fan-out: a node's Directive that spawns N parallel executions of a worker template, N decided at runtime. (Doc 02) |
| **Parley** | A pause point: execution halts at a node, persists a Waypoint tagged as awaiting input, and resumes when a response is supplied. (Doc 03) |
| **Vanguard** | The set of nodes ready to execute in the next superstep (the frontier). (Doc 01) |
| **Chronicle** | The ordered Waypoint history of a thread, supporting replay and forking. (Doc 03) |
| **Aegis** | Per-node fault-tolerance policy bundle: retry + timeout + error handler. (Doc 04) |
| **Vault** | Cross-thread, namespaced long-term memory store (distinct from Garrison conversation history). (Doc 05) |

## 5. Definition of Done (program level)

The program is complete when:

1. Every FR in docs 01–07 is implemented, tested, and passes CI (fmt, clippy `-D warnings`, coverage ≥ 82%, audit, deny).
2. The three end-to-end scenarios in §6 run green as integration tests in `tests/`.
3. All docs (mdBook + rustdoc) updated; `openapi.json` regenerated for new endpoints.
4. `MIGRATION.md` exists at the repository root, follows the structure in §9, and every section is filled in (no "TBD"). It is linked from the README and the mdBook "Upgrading" page.
5. The known-bug fix CF-FR-01 (Custom edge condition) is verified by a failing-then-passing test committed in that order (visible in history or noted in the PR description).
6. The `semver` CI job (X-10.5) and the MSRV CI job (X-11.1) exist, run on every PR, and pass on the release commit; the semver allowlist contains only entries present in `MIGRATION.md` §9.2.
7. All workspace crate versions are `0.10.0`, changelogs updated, and `cargo publish --dry-run` succeeds for every publishable crate in dependency order.

## 6. Program Acceptance Scenarios (must exist as integration tests)

**E2E-1: Crash-resume.** A 6-node cyclic workflow (containing one loop with a max-iteration bound) runs against a mock LLM with a durable Waypoint backend. The test kills the engine (drops it) after superstep 3, constructs a fresh engine from the same backend and `thread_id`, resumes, and asserts: (a) nodes already completed are not re-executed (observed via a call-recording mock port), (b) the final Battlefield equals the Battlefield of an uninterrupted control run, (c) exactly one Waypoint exists per completed superstep.

**E2E-2: Human approval gate.** A workflow reaches a Parley node requesting approval of a destructive action. The test asserts the run returns an `AwaitingInput` outcome (not an error), that the process can be fully dropped and re-created, and that supplying `resume(thread_id, payload)` continues to the correct branch: approval payload "no" routes to a cancellation node; "yes" routes to the action node. Both branches asserted.

**E2E-3: Dynamic map-reduce with per-node fault tolerance.** A planner node's Directive musters N workers (N derived from the planner's mock output, N=5), one worker is configured to fail transiently twice before succeeding (mock), its Aegis retry policy recovers it, a deferred aggregation node runs exactly once after all 5 workers complete, and the Battlefield's list-dispatch field contains exactly 5 worker results in deterministic order.

## 7. Known Bug Fix (mandatory, ships with Doc 02)

**BUG-01:** `CampaignExecutionService::evaluate_edge_condition` currently treats `EdgeCondition::Custom(_)` as always-true with only a log warning. This silently corrupts conditional routing. Doc 02, requirement CF-FR-01 replaces this with a registered-evaluator mechanism and makes an unregistered custom condition a hard `BattalionError::InvalidGraph` at validation time (before any node executes), never a silent pass at runtime.

**BUG-02 (found during phase-one implementation review):** *Silent stranded node.* Because ENG-FR-02 removed toposort (to allow cycles) without adding a replacement connectivity check, a non-entry WarGraph node whose only incoming edges trace back to itself can never become ready; `WarGraph::validate()` accepts the graph and the run reports `RunOutcome::Completed` as if everything executed. Fixed by ENG-FR-02a (reachability-from-entry validation with declared dynamic-reachability exemptions), test-first. Since `WarGraph` is new in v0.10, this is a pre-release engine fix, **not** a v0.9 behavioral change — no `MIGRATION.md` entry or X-10 register row is required; it is in scope for the compatibility audit only as confirmation of that classification.

**BUG-03 (found during the Phase 22 Plan 16 fixture audit):** *Cycle-bootstrap starvation.* `Frontier::is_ready` requires every incoming edge of a node to be resolved before that node is scheduled, and a back-edge into a cycle stays `Pending` because its source can only run *after* the target — `propagate_dead` never marks that source dead, since it is genuinely reachable. So the target of the back-edge can never take its first turn, and the run reports `RunOutcome::Completed` as if everything executed. The self-loop reproduction (a node fed by one upstream edge and its own self-edge) is the minimal instance of the general shape `entry -> a -> b -> a`, where the same starvation blocks `a` regardless of how many distinct nodes form the cycle. BUG-03 is the *runtime* twin of BUG-02's *static* defect: BUG-02 lied about a node no path could reach at all; BUG-03 lies about a node every path reaches, but the readiness rule can never admit into a Vanguard. Fixed by ENG-FR-06a: a starvation-release fallback pass in `compute_next_vanguard` (engaged only when no node is executable by the normal readiness rule or the defer-release rule), plus a validate-time guard and a run-end truthful-outcome check, landed test-first. The `engine` module is absent at the `v0.9.0` tag, so — BUG-02's pre-release classification, verbatim in substance — this is a pre-release engine fix, **not** a v0.9 behavioral change: no `MIGRATION.md` §9.1 entry and no §9.2 X-10 register row is required; it is in scope for the compatibility audit only as confirmation of that classification.

## 8. Out of Scope for This Program

- Porting the engine to other languages (thin HTTP client SDKs are in Doc 06 as optional).
- A full graphical IDE (Doc 07 specifies trace export + minimal visualization only).
- Multi-region/HA storage replication (backends must be pluggable; replication is the backend's concern).
- Billing/metering.

## 9. `MIGRATION.md` — Required Structure and Pre-Populated Content

A single file, `MIGRATION.md`, at the repository root, titled **"Upgrading from v0.9.0 to v0.10.0"**. It is a living document: the implementer creates it in the first epic and appends to it in every epic that touches an item below. The verification pass treats an incomplete or stale `MIGRATION.md` as a program-level failure. Sections and their required content:

### 9.1 Behavioral changes (user-visible without code changes)

Pre-populated entries the implementer MUST complete (add others only via the X-03 stop-and-flag path):

| ID | Change | Who is affected | Required user action |
|---|---|---|---|
| M-B-01 | **BUG-01 fix.** `EdgeCondition::Custom(name)` no longer evaluates to `true` when no evaluator is registered. Campaign/graph validation now fails with `BattalionError::InvalidGraph` naming each unregistered condition, before any node executes. | Any v0.9 workflow using `EdgeCondition::Custom`. Such workflows were routing *incorrectly* (always following the custom edge); after upgrade they fail loudly at validation. | Register an evaluator for each name via the new registry API (CF-FR-01), or replace the condition with `Contains`/`Regex`/`Always`. Include a worked before/after example in this section. |
| M-B-02 | **Graceful shutdown.** On SIGTERM/SIGINT the process now waits up to `shutdown_grace` (default 30 s) for in-flight engine runs to halt at a superstep boundary before exiting. | Operators; container orchestration. | Set `terminationGracePeriodSeconds` ≥ 2 × `shutdown_grace` (k8s manifests under `k8s/` are updated by HITL-FR-15; state the new value). Document how to disable the wait for legacy-only deployments. |
| M-B-03 | **Tool errors fed to the model by default** (RT-FR-24, `tool_error_mode = FeedToModel`). | Users of the Arsenal tool loop who relied on a tool failure aborting the run. | Set `tool_error_mode = FailRun` globally or per tool to restore the previous behavior. *(Implementer: if this default proves too surprising, flip the default to `FailRun` and record the decision here — either way, the chosen default and the rationale are documented.)* |

### 9.2 Rust API changes (compile-affecting; the X-10 register)

A table with columns: **Crate · Type/Trait · Change · Mitigation (`#[non_exhaustive]` / default method / new trait / `#[serde(default)]`) · Deliberate-breaking? (Y/N, justification) · Requirement ID**. Pre-populated rows the implementer MUST resolve (fill the Mitigation and Deliberate columns; add any further pre-existing public types touched):

| Crate | Type / Trait | Change | Req |
|---|---|---|---|
| `paladin-ports` | `StopReason` | new variants `CallLimit`, `TokenBudget` | RT-FR-04, RT-FR-06 |
| `paladin-core` | `BattalionError` | new variant `Node(NodeError)` | FT-FR-02 |
| `paladin-core` | `PaladinError` | new/extended variants carrying HTTP status / transience; new method `transience()` | FT-FR-01 |
| `paladin-llm` / `paladin-ports` | `LlmError` | new/extended variants carrying HTTP status; new variant `AllProvidersFailed`; new method `transience()` | FT-FR-01, FT-FR-16 |
| `paladin-ports` | LLM request struct (the type passed to `LlmPort::generate`) | new optional field `response_format` | RT-FR-17 |
| `paladin-ports` / `paladin-memory` | Garrison entry type | new field `is_summary: bool` (+ SQLite column) | RT-FR-12 |
| `paladin-ports` | `PaladinResult` | new metadata (serving provider from fallback) | FT-FR-17 |
| `paladin-core` | `EdgeCondition` | *no variant change*, but semantics of `Custom` change (see M-B-01) | CF-FR-01 |
| `paladin-battalion` | `Commander` / `CommanderBuilder` | new `StrategySelection` option (additive method; verify no public field added) | CF-FR-19 |
| `paladin-battalion` | `CampaignExecutionService` | new evaluator-registry builder method (constructor unchanged) | CF-FR-01 |
| `paladin-core` | `Waypoint` (new in 0.10; listed for completeness) | `AwaitingInput` carries `Vec<ParleyRequest>` — settle before first release, no compat needed | HITL-FR-03 |

Each row's final state must match the `cargo semver-checks` allowlist exactly (X-10.5).

### 9.3 Toolchain & dependencies

- Final MSRV (1.85 unless bumped per X-11.2), with the reason if bumped.
- New crates added, grouped by workspace crate and feature flag; confirmation that the default feature set of `paladin-ai` did not gain heavyweight dependencies (X-11.4).
- New cargo features introduced (`postgres`, `otel`, `llm-gemini`, `dev-ui`, …) with one-line descriptions and what each pulls in.

### 9.4 Persistence & schema migrations

For every new or altered table (Waypoints, run traces, runs, assistants, schedules, webhook deliveries, vault, Garrison `is_summary` column): migration file name, backend(s), whether it runs automatically at startup or must be applied manually, reversibility, and expected storage growth guidance (link to ENG-FR-18 retention config). Explicit statement that existing Citadel JSON/SQLite state files remain readable unchanged.

### 9.5 Configuration & environment

Every new config struct and every new `APP_*` environment variable (X-09), with defaults, and a note that all new subsystems are **disabled by default** so a v0.9 configuration file boots v0.10 with identical behavior (this claim MUST be backed by an integration test that boots the server with the v0.9 sample config and asserts feature flags/config resolve to the legacy behavior).

### 9.6 HTTP API

Confirmation that every v0.9 endpoint's path, request schema, response schema, and status codes are unchanged (backed by a golden diff of `openapi.json` restricted to pre-existing paths), plus the list of new endpoints with a pointer to the regenerated spec.

### 9.7 Deprecations

Every item marked `#[deprecated]` in this release, its replacement, and the earliest release in which it may be removed (not before v0.11.0).

### 9.8 Upgrade checklist

A short, ordered, copy-pasteable checklist for operators: back up state dirs/DBs → apply migrations → update config (nothing required for legacy behavior) → adjust termination grace → register custom evaluators if used → deploy → verify with `paladin-cli` health/graph-validate commands.
