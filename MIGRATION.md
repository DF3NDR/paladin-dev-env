# Upgrading from v0.9.0 to v0.10.0

> **Scope note:** this file is the root Markdown deliverable required by `.project/v0.10.0/00-program-overview.md`
> §9. It is a **living document**, created in the first epic of the v0.10.0 program (Phase 22,
> `ENG-08`) and appended to by every later epic that touches an item below. The mdBook "Upgrading"
> page the program's Definition of Done (overview §5.4) also requires is **not** added by this
> phase — that is Phase 29 / `SHIP-01` scope. Every `TBD` below carries the requirement or phase
> that owns closing it; `SHIP-01` (Phase 29) is responsible for clearing every remaining `TBD`
> before the v0.10.0 release, per overview §9's living-document contract.

## 9.1 Behavioral changes (user-visible without code changes)

| ID | Change | Who is affected | Required user action |
|---|---|---|---|
| M-B-01 | **BUG-01 fix.** `EdgeCondition::Custom(name)` no longer evaluates to `true` when no evaluator is registered. Campaign/graph validation now fails with `BattalionError::InvalidGraph` naming each unregistered condition, before any node executes. | Any v0.9 workflow using `EdgeCondition::Custom`. Such workflows were routing *incorrectly* (always following the custom edge); after upgrade they fail loudly at validation. | Register an evaluator for each name via the new registry API (CF-FR-01), or replace the condition with `Contains`/`Regex`/`Always`. |
| M-B-02 | **Graceful shutdown.** On SIGTERM/SIGINT the process now waits up to `shutdown_grace` (default 30 s, env `APP_ENGINE_SHUTDOWN_GRACE_SECS`) for in-flight engine runs to halt before exiting — `paladin-server`'s `shutdown_signal` and `ServiceRunner::wait_for_shutdown` both cancel a shared `ShutdownCoordinator` and wait for registered runs to drain (HITL-04, D-21/D-22). | Operators; container orchestration. | Set `terminationGracePeriodSeconds` ≥ 2 × `shutdown_grace` — `k8s/server/deployment.yaml` and `k8s/deployment.yaml` ship `60` (landed, HITL-FR-15). Set `APP_ENGINE_GRACEFUL_SHUTDOWN=false` to restore the old no-wait behavior for legacy-only deployments (landed, HITL-04). |
| M-B-03 | **Tool errors fed to the model by default** (RT-FR-24, `tool_error_mode = FeedToModel`). | Users of the Arsenal tool loop who relied on a tool failure aborting the run. | Set `tool_error_mode = FailRun` globally or per tool to restore the previous behavior. |
| M-B-04 | **Automatic per-superstep checkpointing.** Any graph executed through the new `WarEngine` (via `WarGraph`/`WarEngine::start`/`WarEngine::resume`) now writes one `Waypoint` — a **full `Battlefield` snapshot**, including whatever a workflow places in shared state (which may include raw LLM prompts and model outputs) — after every superstep, by default (`WaypointDurability::Strict`). The write goes to whichever `WaypointPort` backend the caller wires in: `InMemoryWaypointStore` (ungated, tests/dev) or, once landed, `SqliteWaypointStore`/`PostgresWaypointStore` (ENG-05) for durable deployments. Growth is bounded by `EngineLimits` (`max_supersteps` default 50, `max_node_visits` default 25) and, once configured, by `WaypointRetentionConfig` (`max_age_days`, `max_waypoints_per_thread` — see §9.5). | Any workflow author who adopts the new `WarEngine`/`WarGraph` APIs (ENG-01…ENG-08). | **Legacy `FormationExecutionService`, `PhalanxExecutionService`, `CampaignExecutionService`, and `Commander` execution paths are completely unaffected — they write no Waypoints and their behavior is byte-for-byte unchanged (ENG-FR-20).** A v0.9 workflow gains **no new persistence** unless it is explicitly rebuilt against the new engine. To adopt it: choose a `WaypointPort` backend, review `WaypointDurability` (default `Strict` fails the run on a write error; `BestEffort` downgrades to a logged warning), and configure `WaypointRetentionConfig` if snapshots — which may contain raw prompts/outputs — must not accumulate unbounded. |

**Worked examples** (owed alongside the rows above, per D-08's rule that a pending item is acceptable only in later-epic-owned content):

- M-B-01: **landed (CF-01, Phase 23).** A v0.9 campaign edge like

  ```rust
  // v0.9 — this edge silently fired on EVERY run, regardless of what
  // `analyzer` actually output, because `EdgeCondition::Custom` had no
  // registration mechanism and evaluated as always-true (BUG-01).
  campaign.add_edge(CampaignEdge::new(
      analyzer_id,
      escalate_id,
      EdgeCondition::Custom("is_urgent".to_string()),
  ))?;
  let service = CampaignExecutionService::new(paladin_port);
  ```

  now fails loudly at validation unless an evaluator is registered for
  `"is_urgent"`. The v0.10 fix registers one on the service (the same
  `CampaignEdge`/`EdgeCondition::Custom` construction is unchanged):

  ```rust
  use paladin_battalion::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};

  struct IsUrgent;

  #[async_trait::async_trait]
  impl EdgeConditionEvaluator for IsUrgent {
      async fn evaluate(&self, output: &str, _ctx: &EdgeContext<'_>) -> Result<bool, EdgeEvaluatorError> {
          Ok(output.to_lowercase().contains("urgent"))
      }
  }

  let service = CampaignExecutionService::new(paladin_port)
      .with_evaluator("is_urgent", Arc::new(IsUrgent));
  ```

  The `WarEngine` path registers the same evaluator via
  `WarEngine::with_edge_evaluator("is_urgent", Arc::new(IsUrgent))`.
  A user who does not want an evaluator at all can instead replace the
  condition with `EdgeCondition::Contains("urgent".to_string())`,
  `EdgeCondition::Regex(...)`, or `EdgeCondition::Always` — no registry
  entry needed for any of the three. Skipping both routes produces this
  exact validation error, transcribed verbatim from the shipped code
  (`BattalionError::InvalidGraph`'s `Display`, legacy path; the
  `WarEngine` path's `EngineError::UnregisteredEdgeCondition` names the
  same offending set):

  ```text
  Invalid graph: unregistered custom edge condition(s): is_urgent -- register with CampaignExecutionService::with_evaluator before calling execute
  ```
- M-B-02: **landed (HITL-04, Phase 24).** Before this phase, SIGTERM/SIGINT exited the process
  immediately — any in-flight superstep run was dropped mid-flight with no chance to reach a
  Halted boundary, and neither shipped manifest declared `terminationGracePeriodSeconds`:

  ```yaml
  # v0.9 — k8s/server/deployment.yaml's pod spec (no termination grace declared;
  # the kubelet's own default of 30s applied, with no relationship to any
  # engine-level grace concept because none existed)
  spec:
    template:
      spec:
        containers:
          - name: paladin-server
            # ...
  ```

  After this phase, the process waits up to a configured grace window for
  in-flight runs to drain before exiting, and both shipped manifests declare a
  termination grace derived from — and at least twice — that window:

  ```yaml
  # v0.10 — k8s/server/deployment.yaml's pod spec
  spec:
    template:
      spec:
        # 2x the default APP_ENGINE_SHUTDOWN_GRACE_SECS (30s) so the kubelet's
        # SIGKILL deadline never lands mid-drain.
        terminationGracePeriodSeconds: 60
        containers:
          - name: paladin-server
            # ...
  ```

  Two new env vars (`EngineConfig`, `src/config/engine.rs`) control the process-level wait:

  | Env var | Default | Effect |
  |---|---|---|
  | `APP_ENGINE_SHUTDOWN_GRACE_SECS` | `30` | Seconds `paladin-server`/`ServiceRunner` wait, after SIGTERM/SIGINT, for in-flight superstep runs to finish before giving up on the stragglers. |
  | `APP_ENGINE_GRACEFUL_SHUTDOWN` | `true` | Set `false` to restore the v0.9 no-wait behavior for legacy-only deployments — the process cancels in-flight runs and exits immediately without waiting. |

  An operator upgrading with no config changes gets `shutdown_grace_secs=30`,
  `graceful_shutdown=true` and `terminationGracePeriodSeconds: 60` from the shipped
  manifests — a strictly safer default than v0.9's immediate exit, not a behavior an operator
  must opt into. A run still executing when SIGTERM arrives either finishes inside the grace
  window (Waypoint records completion normally) or is aborted at the deadline (`NodeOutcomeKind::
  Skipped { reason: "shutdown" }`, re-listed in the Halted Waypoint's vanguard for exactly-once
  resume) — no in-flight work silently vanishes either way.
- M-B-03: TBD — owner RT-07, Phase 26. A concrete `tool_error_mode` before/after example lands when RT-07 ships the tool-loop default and records its rationale here.
- M-B-04: no worked example owed — this phase (ENG-08) both introduces the behavior and documents it in full above.

## 9.2 Rust API changes (compile-affecting; the X-10 register)

Columns: **Crate · Type/Trait · Change · Mitigation (`#[non_exhaustive]` / default method / new trait / `#[serde(default)]`) · Deliberate-breaking? (Y/N, justification) · Requirement ID**.

| Crate | Type / Trait | Change | Mitigation | Deliberate-breaking? | Req |
|---|---|---|---|---|---|
| `paladin-ports` | `StopReason` | new variants `CallLimit`, `TokenBudget` | TBD — owner RT-02, Phase 26 | TBD — owner RT-02, Phase 26 | RT-FR-04, RT-FR-06 |
| `paladin-core` | `BattalionError` | new variant `Node(NodeError)` | TBD — owner FT-01, Phase 25 | TBD — owner FT-01, Phase 25 | FT-FR-02 |
| `paladin-core` | `PaladinError` | new/extended variants carrying HTTP status / transience; new method `transience()` | TBD — owner FT-01, Phase 25 | TBD — owner FT-01, Phase 25 | FT-FR-01 |
| `paladin-llm` / `paladin-ports` | `LlmError` | new/extended variants carrying HTTP status; new variant `AllProvidersFailed`; new method `transience()` | TBD — owner FT-01 (transience), FT-05 (`AllProvidersFailed`), Phase 25 | TBD — owner FT-01, FT-05, Phase 25 | FT-FR-01, FT-FR-16 |
| `paladin-ports` | LLM request struct (the type passed to `LlmPort::generate`) | new optional field `response_format` | TBD — owner RT-05, Phase 26 | TBD — owner RT-05, Phase 26 | RT-FR-17 |
| `paladin-ports` / `paladin-memory` | Garrison entry type | new field `is_summary: bool` (+ SQLite column) | TBD — owner RT-03, Phase 26 | TBD — owner RT-03, Phase 26 | RT-FR-12 |
| `paladin-ports` | `PaladinResult` | new metadata (serving provider from fallback) | TBD — owner FT-05, Phase 25 | TBD — owner FT-05, Phase 25 | FT-FR-17 |
| `paladin-core` | `EdgeCondition` | *no variant change*, but semantics of `Custom` change (see M-B-01) | none required — no signature or variant change; the semantic change is M-B-01 | N | CF-FR-01 |
| `paladin-battalion` | `Commander` / `CommanderBuilder` | new `StrategySelection` option (additive method; verify no public field added) | additive builder method (`CommanderBuilder::strategy_selection`); the new `StrategySelection` option is stored in a private `Commander` field, defaulting to `Heuristic` -- no public field added, verified by source assertion (`awk`-scanned `pub struct Commander` block) | N | CF-FR-19 |
| `paladin-battalion` | `CampaignExecutionService` | new evaluator-registry builder method (constructor unchanged) | additive builder method (`with_evaluator`); `new(paladin_port)` unchanged | N | CF-FR-01 |
| `paladin-core` | `Waypoint` (new in 0.10; listed for completeness) | `AwaitingInput` carries `Vec<ParleyRequest>` — settle before first release, no compat needed | N/A — new type introduced in v0.10, not a pre-existing public API. Stub landed in Plan 22-01; payload finalized by HITL-01, Phase 24. | N/A (new type; X-10 governs only pre-existing types) | HITL-FR-03 |

**Note on Plan 22-01 (this phase's tracer plan):** Plan 22-01 added the `battlefield`, `battlefield_error`, and `waypoint` modules to `paladin-core`, `waypoint_port` to `paladin-ports`, the `waypoint` adapter module to `paladin-storage`, and the `engine` module to `paladin-battalion`. All of these are **new** modules/types — none of Plan 22-01's changes touched a *pre-existing* public type's signature (its "modified" files were module-registration edits, e.g. adding a `mod battlefield;` line). **This is a deliberate zero**, not an omission: no register row is added for Plan 22-01.

**Note on Phase 23 (Control Flow — Dynamic Routing, Fan-Out & Subgraphs), D-27:** this phase reshapes several engine and Waypoint types this plan's own read-list names — `StateNode` (`run`'s return type changed from a bare `StateDelta` to `Directive`), `NodeSpec` (a `Battalion` variant and, on `Paladin`, a `directive_parser` field added), `NodeContext` (a `muster: Option<MusterContext>` field added), `EngineLimits` (a `max_muster_tasks` field added), `EngineError` (new variants added to an already `#[non_exhaustive]` enum — `GotoUnknownNode`, `ParleyNotSupported`, `DirectiveParseFailed`, `EmptyMuster`, `DuplicateMusterTaskKey`, `MusterTaskLimitExceeded`, `MusterUnknownWorker`, `MusterWorkerNotATemplate`, `WorkerTemplateIsEntry`, `WorkerTemplateHasStaticIncomingEdge`, `MusterPrefixSchemaField` among them), `WarGraph` (`validate`'s signature changed — it gained an `edge_evaluators: &EdgeEvaluatorRegistry` parameter for CF-01's fail-closed unregistered-condition check — and a new `worker_templates` marker set was added, seeded via the new `add_worker_template` method) and `Waypoint` (two additive `#[serde(default)]` fields carrying Muster progress and `checkpoint_ns`). Every one of these types is **absent at the `v0.9.0` tag** — `StateNode`, `NodeSpec`, `NodeContext`, `EngineLimits` and `EngineError` were all introduced by Plan 22-01 (see the note immediately above); `Waypoint` is new in 0.10 and already carries the "N/A — new type" row above. Because none of them is a *pre-existing* public type, X-10 — which governs only a signature change to a type that shipped in `v0.9.0` — does not apply to any of them, and none gets its own §9.2 row here. **This is a deliberate zero**, not an omission: a reader who diffs this phase's changes against a type list and finds no `StateNode`/`NodeSpec`/`NodeContext`/`EngineLimits`/`EngineError`/`WarGraph`/`Waypoint` row should read that absence as this note, not as an oversight.

## 9.3 Toolchain & dependencies

- **Final MSRV: 1.88** (Rust, edition 2024) — an X-11.2 stop-and-flag resolution, landed in Plan 22.1-04. The prior "1.85" figure was derived from edition 2024's own minimum and was never measured against this workspace's actual dependency graph; 1.88 is the first MSRV figure in this project's history with a passing measurement behind it (`cargo +1.88 check --workspace --all-features --all-targets --locked`, zero errors). The chain that forces it: the STDIO MCP transport requires `rmcp` at its exact `=2.1.0` pin (D-06, unchanged — no `rmcp` version restores 1.85 with `transport-child-process`), which with the `transport-child-process` feature requires `process-wrap ^9.0`, whose releases require rustc 1.86 or 1.87; separately, `time` >= 0.3.47 (the fix for RUSTSEC-2026-0009) requires rustc 1.88. The higher of the two — 1.88 — is the floor, and it clears the advisory with no new security exception. Two alternatives were considered and rejected: **1.87 plus a new RUSTSEC-2026-0009 exception** (rejected — adds to the exception sprawl ADR-0024 governs, for a floor one release lower than the clean option), and **1.86 with `process-wrap` held at an exact `.0` pin** (rejected — a transitive `.0` pin plus the same exception). The declared floor now lives in exactly one place, `workspace.package.rust-version = "1.88"` in root `Cargo.toml`; all ten crate manifests inherit it via `rust-version.workspace = true`. The lockfile was restored to its pre-`44e13fbd` resolution (reverting the 22-17 hand-downgrade of `time`, `process-wrap`, `darling`, `serde_with`, `tonic`, `idna_adapter`, `home` and others) and re-resolved under the new `[workspace] resolver = "3"` (D-09), which also serves as the recurrence guard against a future `cargo update` silently re-selecting a version above the declared floor. **X-11.4 default-feature-set confirmation:** `cargo tree -e features -p paladin-ai` under default features was compared before (HEAD's pinned-back lockfile) and after (the restored lockfile) this change — the only package-name delta in the full workspace lockfile is `bs58` reappearing (a resolution-only difference restored alongside the rest of the pre-`44e13fbd` graph, per CONTEXT.md's discretion note), and `bs58` does not appear anywhere in `paladin-ai`'s default-feature tree. The default set did not grow.
- **New feature added to an existing dependency — `uuid`'s `v7` feature.** Added to the existing pinned `uuid = 1.8.0` workspace dependency (D-03) to generate time-ordered `WaypointId`s via `Uuid::now_v7()`. No new crate name. Landed in Plan 22-01 (`Cargo.toml`'s `[workspace.dependencies] uuid` feature array now reads `["v4", "v7", "serde"]`).
- **New cargo feature — `postgres` on `paladin-storage`. Landed this plan (22-06), Task 2.** `postgres = ["dep:sqlx", "sqlx/postgres"]` (mirroring the existing `sqlite`/`mysql` feature-gate pattern) enabling `PostgresWaypointStore` (ENG-05/D-01), plus the facade passthrough `storage-postgres = ["paladin-storage/postgres"]` on `paladin-ai` (mirroring the existing `storage-mysql` passthrough) — `storage = ["storage-mysql", "storage-postgres"]` — so a consumer of the facade crate reaches it via `cargo build -p paladin-ai --features storage-postgres` without any `paladin-storage`-level override. Neither `default` nor this addition to `storage`/`full` puts Postgres in the default set.
- **`sqlx`'s `macros` feature added — Task 1 blocking-issue fix.** `sqlx::migrate!` (used by both `sqlite.rs` and `postgres.rs` to embed their versioned migrations at compile time) lives behind sqlx's `macros` feature, not `migrate` alone; added to the workspace-level `sqlx` dependency (`Cargo.toml`'s `[workspace.dependencies] sqlx` feature array now reads `[..., "migrate", "macros"]`). No new crate name — same already-pinned `sqlx 0.8`.
- **Default feature set impact (X-11.4), confirmed at this plan's HEAD:** `cargo tree -e features -p paladin-ai` (default features) contains no `sqlx/postgres`/Postgres-driver entry anywhere in the graph — verified by `cargo tree -e features -p paladin-ai | grep -i postgres` returning no matches. `cargo build -p paladin-ai --features storage-postgres` compiles the adapter; `cargo build -p paladin-storage --features postgres` and `cargo clippy -p paladin-storage --features postgres --all-targets -- -D warnings` are both clean. The default `paladin-ai` build also gains no heavyweight dependency from the earlier `uuid`/`v7` addition — `v7` only pulls in `uuid`'s own `atomic`/`rng` sub-features, not a new crate.

## 9.4 Persistence & schema migrations

- **New table: `waypoints`. Landed this plan (22-06).** Two versioned migration files, one per backend (superseding this section's earlier single-path placeholder from Plan 22-04 — one crate now ships two dialects of the same logical table, hence the per-backend subdirectory split): `crates/paladin-storage/migrations/sqlite/001_create_waypoints_table.sql` (Task 1) and `crates/paladin-storage/migrations/postgres/001_create_waypoints_table.sql` (Task 2; `001_` continues the per-crate numbering convention established by `crates/paladin-memory/migrations/001_create_garrison_tables.sql`). SQLite stores `payload` as `TEXT` holding serialized JSON (D-02, for debuggability and because the `sqlx` `json` feature is already enabled workspace-wide); PostgreSQL stores `payload` as `JSONB`, bound via an explicit `::jsonb` cast on the parameter (sqlx sends bound `String` parameters as text over the wire; the cast is what tells Postgres to store it as JSONB rather than TEXT). Both share the same logical schema: `waypoint_id` (primary key), `thread_id`, `parent_id` (nullable), `superstep`, `status` (its own JSON-serialized column, so `history`/`list_threads` summaries can be built without deserializing the full payload), `payload`, `created_at`, with an index on `(thread_id, created_at DESC)`.
- **Runs automatically, not manually.** Both migrations are embedded at compile time via `sqlx::migrate!` (which required adding `sqlx`'s `macros` feature to the workspace-level `sqlx` dependency — a Task 1 blocking-issue fix, no new crate name, see §9.3) and applied automatically when `SqliteWaypointStore`/`PostgresWaypointStore` is constructed — no manual `sqlx migrate run` step is required of the operator. Idempotent: constructing a store twice against the same database is safe (`CREATE TABLE IF NOT EXISTS`/`CREATE INDEX IF NOT EXISTS` plus `sqlx::migrate::Migrator`'s own applied-version tracking), proven by `constructing_store_twice_against_same_file_is_idempotent` (SQLite) and exercised against a real server by the Postgres Tier 2 suite (`docker/docker-compose.test.yml`'s `postgres-test` service, `make test-integration-docker`).
- **Reversibility:** not reversible without data loss (dropping `waypoints` discards all checkpoint history). No `down` migration is provided, consistent with every existing migration in this workspace (`garrison`, `workflow`, `content`, `user` tables), none of which ship a reverse migration.
- **Storage growth guidance:** unbounded by default — every completed superstep writes one `waypoints` row containing a full `Battlefield` snapshot. Configure `WaypointRetentionConfig` (`max_age_days`, `max_waypoints_per_thread`; see §9.5) to bound growth. Pruning MUST NOT (and, per ENG-FR-18, will not) delete a thread's single latest Waypoint or any Waypoint with status `AwaitingInput`.
- **Memory and storage growth are measured, not asserted (ENG-NFR-01/02, Plan 22-10).** `benches/engine_benchmarks.rs` measures `SqliteWaypointStore::save` latency at three Battlefield payload sizes (1 KiB, 512 KiB, just under 1 MiB) against the ENG-NFR-01 target of under 10 ms p50, and per-superstep engine wall-clock cost at two Vanguard widths; `examples/war_engine_memory_baseline.rs` measures resident-memory growth per superstep for a fixed graph and counts Battlefield clones per superstep (ENG-NFR-02: at most one shared clone per superstep, via the Arc-shared read snapshot, plus one per concurrently executing node view), failing loudly if that bound regresses. Measured figures and the commit they were taken at are recorded in `.planning/phases/22-battlefield-state-superstep-engine/22-10-SUMMARY.md` — treat the per-checkpoint storage-growth guidance above alongside those figures, not in isolation, when sizing a deployment's Waypoint retention policy.
- **Existing state is unaffected.** Existing Citadel JSON/SQLite state files remain readable unchanged — the `waypoints` table is entirely new and additive. `WaypointPort` is deliberately separate from `CitadelPort` (see the rustdoc rationale on `crates/paladin-ports/src/output/waypoint_port.rs`: Waypoints are high-frequency and append-mostly, addressed by thread; Citadel remains for coarse, explicitly-invoked whole-entity snapshots). No Citadel schema or file format is touched by this phase.
- **Persisted `GraphFingerprint` format.** Every Waypoint's payload embeds a `GraphFingerprint` string in the form `{version}:{blake3_hex}` — a versioned tag over the blake3 hex digest of the canonical, deterministically sorted byte stream of node ids (plus Paladin output fields), edge specs, schema field dispatch rules, the declared entry set, `defer_flags`, and `dynamic_targets` (excludes prompts/models, per D-04). This format was decided at Plan 22-01's Task 1 checkpoint (option-b), specifically so a future algorithm change is a detectable new tag rather than a silent invalidation of every stored thread's `resume`. **Bumped `v1:` -> `v2:` in Phase 22.1 (CR-01, D-17):** `v1`'s canonical byte encoding concatenated fields with unescaped ASCII delimiters (`|`, `-`, `:`) that were themselves legal `NodeId`/`FieldName` characters, so two structurally different graphs could be crafted to hash identically (e.g. two nodes `"a"`+`"b"` vs. one node `"a|nf|b"`). `v2` replaces this with a length-prefixed encoding with no delimiter collision. Every `v1:`-tagged fingerprint stored before this bump is recognized as stale (a version-tag mismatch) rather than silently reinterpreted under the new encoding.

## 9.5 Configuration & environment

Every new runtime behavior this phase introduces is tunable and **disabled or defaulted to today's behavior out of the box** (X-09), so a v0.9 configuration file boots v0.10 with identical behavior. This claim will be backed by an integration test that boots the server with the v0.9 sample config and asserts feature/config resolution to legacy behavior — TBD, owner SHIP-02, Phase 29.

Config structs mirror the existing `CitadelConfig` shape at `src/config/citadel.rs`: `Default` + `validate()` + `EnvOverridable`.

- **`EngineConfig`** (`src/config/engine.rs`) — **landed this plan (23-07, Task 1)**:
  - `max_supersteps: u64` — default `50`. Env: `APP_ENGINE_MAX_SUPERSTEPS`.
  - `max_node_visits: u32` — default `25`. Env: `APP_ENGINE_MAX_NODE_VISITS`.
  - `run_timeout_secs: Option<u64>` — default `None` (plumbing-only this phase; Doc 04/`FT-03` owns timeout semantics). Env: `APP_ENGINE_RUN_TIMEOUT_SECS`.
  - `waypoint_durability: WaypointDurability` (`Strict` | `BestEffort`) — default `Strict`. Env: `APP_ENGINE_WAYPOINT_DURABILITY`.
  - `max_muster_tasks: u32` — default `100` (CF-FR-13, D-16). Env: `APP_ENGINE_MAX_MUSTER_TASKS`. `validate()` rejects `0`.
  - `shutdown_grace_secs: u64` — default `30` (HITL-04, D-20). Env: `APP_ENGINE_SHUTDOWN_GRACE_SECS`. Seconds a mid-superstep cancellation races the in-flight batch of spawned node tasks against before aborting stragglers; also the upper bound `paladin-server`/`ServiceRunner` wait on SIGTERM/SIGINT before exiting. `validate()` rejects values above `3600` (1 hour). **Landed this plan (24-08).**
  - `graceful_shutdown: bool` — default `true` (HITL-04, D-20). Env: `APP_ENGINE_GRACEFUL_SHUTDOWN`. Set `false` to skip the shutdown-grace wait entirely and exit immediately on SIGTERM/SIGINT — the M-B-02 disable switch for legacy-only deployments (§9.1). **Landed this plan (24-08).**
  - Converts into `EngineLimits`/`WaypointDurability` via `impl From<EngineConfig> for EngineLimits` (`src/config/engine.rs`); `waypoint_durability` is read directly off the source `EngineConfig` value and passed to `WarEngine::with_durability` separately. `shutdown_grace_secs`/`graceful_shutdown` are deliberately excluded from this conversion — both are runtime-only settings, never hashed into `WarGraph::fingerprint()` (proven by `shutdown_grace_does_not_change_the_graph_fingerprint`).
  - Both new fields default to today's absent behavior (an unconfigured v0.9 deployment gets `shutdown_grace_secs=30`, `graceful_shutdown=true` — the same values a v0.10 deployment that never mentions either env var gets), so upgrading changes nothing about a workflow's own behavior; it only changes what the *process* does on SIGTERM/SIGINT (see M-B-02, §9.1).
- **`WaypointRetentionConfig`** (`src/config/waypoint_retention.rs`) — **landed this plan (22-06, Task 3)**:
  - `enabled: bool` — default `false` (no pruning runs until an operator opts in — the new subsystem is off by default). Env: `APP_WAYPOINT_RETENTION_ENABLED`.
  - `max_age_days: Option<u32>` — default `None`. Env: `APP_WAYPOINT_RETENTION_MAX_AGE_DAYS`. `validate()` rejects `Some(0)`.
  - `max_waypoints_per_thread: Option<u32>` — default `None`. Env: `APP_WAYPOINT_RETENTION_MAX_WAYPOINTS_PER_THREAD`. `validate()` rejects `Some(0)`.
  - This config is driven through `WaypointRetentionService` (`src/application/services/waypoint_retention.rs`, Plan 22-14), which holds the one definition of "protected" — a thread's latest Waypoint plus every `AwaitingInput` Waypoint (ENG-FR-18, T-22-21) — and hands it to `paladin_storage::waypoint::retention::prune` as an argument. The routine itself no longer decides what protected means; it unions whatever it is handed with the configured bounds and removes the rest through `WaypointPort::prune_thread` (Plan 22-13), a keep-set primitive that never touches a protected id — not the delete-then-resave sequence this section originally described. Monotone, crash-safe, and idempotent by construction; runs unchanged over `InMemoryWaypointStore`, `SqliteWaypointStore`, and `PostgresWaypointStore`.

Since every new field either defaults to today's absent behavior (`enabled: false`, no retention pruning) or to a bounded-but-generous limit that only applies to the *new* `WarEngine` path (legacy Formation/Phalanx/Campaign call sites never construct an `EngineConfig`), a v0.9 configuration file — which has no `engine:`/`waypoint_retention:` section at all — resolves to identical runtime behavior: `EngineConfig::default()` converts to exactly `EngineLimits::default()` and `WaypointDurability::Strict`, asserted mechanically by `config::engine::tests::default_engine_config_matches_todays_engine_defaults` rather than left as an unchecked claim.

- **`LlmDecision` and Commander `StrategySelection::Semantic` add no config struct or environment variable (CF-05, D-26).** Both hold `Arc<dyn LlmPort>` trait objects assembled and passed in code — an `LlmDecisionEvaluator` registered by name through `EdgeEvaluatorRegistry`; a `StrategySelection::Semantic { llm, model }` passed to `CommanderBuilder::strategy_selection` — never resolved through an `APP_*` environment variable or a config-file section, so a v0.9 deployment gains no new outbound, paid, non-deterministic model call merely by upgrading (`grep -rn 'APP_LLM_DECISION\|APP_.*SEMANTIC' src/ crates/` returns no matches). `on_ambiguous` (per-`LlmDecisionEvaluator`) and `on_parse_error` (per-node `DirectiveParser`, CF-02) are likewise per-evaluator/per-node enums set in code, not runtime-global tunables — X-09 requires a config struct only for a *tunable*, and neither of these is one.

## 9.6 HTTP API

TBD — no HTTP surface is added or changed by this phase (Phase 22 ships no engine-backed endpoints; see CONTEXT.md's phase boundary). Owner for the two halves of this section: the golden `openapi.json` diff proof that pre-existing `/v1` paths are unchanged is SHIP-02, Phase 29; the list of new engine-backed endpoints (`GET /threads/{id}/state`, `POST /threads/{id}/resume`, `GET /threads/{id}/history`) is HITL-05, Phase 24, with the broader platform surface (background runs, assistants, schedules) under PLAT-01…PLAT-06, Phase 27.

## 9.7 Deprecations

TBD — no item is marked `#[deprecated]` by this phase. Entries are added here as producing epics ship theirs; this section is finalized (or confirmed empty) at closeout, owner SHIP-01, Phase 29.

## 9.8 Upgrade checklist

TBD — a full ordered, copy-pasteable operator checklist (back up state dirs/DBs → apply migrations → update config → adjust termination grace → register custom evaluators if used → deploy → verify with `paladin-cli` health/graph-validate commands) is written once the subsystems it references exist to check against (migrations land with ENG-05; termination grace **landed with HITL-04, Phase 24** — set `terminationGracePeriodSeconds` to at least `60` (2 × the 30s default `APP_ENGINE_SHUTDOWN_GRACE_SECS`), or at least twice whatever value you configure that env var to, in every Deployment manifest before rolling out this upgrade; custom evaluator registration **landed with CF-01, Phase 23** — `CampaignExecutionService::with_evaluator` on the legacy path, `WarEngine::with_edge_evaluator` on the `WarEngine` path, see M-B-01). Finalized at closeout, owner SHIP-01, Phase 29.
