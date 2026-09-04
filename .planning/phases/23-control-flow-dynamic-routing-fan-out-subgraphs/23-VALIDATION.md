---
phase: 23
slug: control-flow-dynamic-routing-fan-out-subgraphs
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: false
wave_0_complete: true
created: 2026-09-03
validated: 2026-09-04
---

# Phase 23 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust, MSRV 1.88); async tests via `#[tokio::test]` including `flavor = "multi_thread"` for the muster stress tests |
| **Config file** | `Cargo.toml` (workspace root; `[[test]]` targets `subgraph_formation_in_campaign` and `e2e_muster_defer_order` registered by 23-09 / 23-11), `Makefile` (`test`, `test-all`, `test-integration-docker`) |
| **Quick run command** | `cargo test -p paladin-battalion --lib` |
| **Full suite command** | `cargo test --workspace --lib --bins && cargo test --test e2e_crash_resume --test e2e_muster_defer_order --test subgraph_formation_in_campaign --test war_engine_tracer` |
| **Estimated runtime** | Quick: ~4 s warm (475 tests). Full: ~2–3 min warm; 15–25 min from a cold `target/` |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p paladin-battalion --lib`
- **After every plan wave:** Run the full suite command above, plus `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds warm (the `paladin-ai` `config::engine` run recompiles the root crate: 26 s measured)

---

## Per-Task Verification Map

All commands below were re-run live on 2026-09-04 at `a45d8514`. Test counts are the `passed` figure from that run. Commands marked *(corrected)* differ from the PLAN's `<automated>` block — see the audit trail for why.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 23-01-01 | 01 | 1 | CF-01 | T-23-01, T-23-03, T-23-04 | Unregistered `Custom(name)` fails validation before any node executes on both paths; evaluator error fails the run; `EdgeEvaluatorFailed` carries no Battlefield data; RED commit precedes GREEN | unit | `cargo test -p paladin-battalion --lib` | ✅ | ✅ green (475) |
| 23-01-02 | 01 | 1 | CF-01 | T-23-04 | MIGRATION §9.1 M-B-01 worked example present; §9.2 CF-01 rows resolved | doc-smoke | `grep -q 'with_evaluator' MIGRATION.md && ! grep -q 'TBD — owner CF-01' MIGRATION.md` *(corrected)* | ✅ | ✅ green |
| 23-02-01 | 02 | 2 | CF-02 | T-23-05, T-23-06 | `Goto` to an undeclared node fails at Directive receipt; unbounded Goto loop trips `max_node_visits` | unit | `cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-02-02 | 02 | 2 | CF-02 | T-23-07, T-23-08 | `End` beats `Goto` in the same superstep; starvation check still fires when no node ended the run; `Parley` is a typed `ParleyNotSupported` | unit | `cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-03-01 | 03 | 2 | CF-05 | T-23-09, T-23-11, T-23-12 | Errors carry a fixed `llm_error_class`, never prompt/response text; one LLM call per decision per superstep; unmatched answer resolves through `on_ambiguous` (default `Fail`) | unit | `cargo test -p paladin-battalion --lib llm_decision` | ✅ | ✅ green (9) |
| 23-03-02 | 03 | 2 | CF-05 | T-23-10, T-23-13 | `StrategySelection::default()` is `Heuristic`; Semantic falls back to Heuristic on any LLM error with the reason recorded and the response body excluded; `Debug` prints a placeholder; 52 pre-existing `test_*` Commander tests unmodified (`git diff 38abd07f..HEAD` removes no `fn test_`) | unit | `cargo test -p paladin-battalion --lib commander` | ✅ | ✅ green (57) |
| 23-04-01 | 04 | 3 | CF-02 | T-23-14, T-23-15 | Envelope delta naming an unknown Battlefield field fails the run; a parsed `Goto` passes the same `GotoUnknownNode` check | unit | `cargo test -p paladin-battalion --lib engine::directive_parser` | ✅ | ✅ green (10) |
| 23-04-02 | 04 | 3 | CF-02 | T-23-16, T-23-17 | `#[serde(deny_unknown_fields)]` on the envelope; `DirectiveParser::default() == PlainOutput`; `on_parse_error` default fails the run | unit | `cargo test -p paladin-battalion --lib engine::directive_parser` | ✅ | ✅ green |
| 23-05-01 | 05 | 4 | CF-03 | T-23-19, T-23-20 | Each worker sees only its own payload; payload never enters the Battlefield; deltas merge in `task_key` order, not completion order | unit | `cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-05-02 | 05 | 4 | CF-03 | T-23-18, T-23-21, T-23-22 | Duplicate key, over-limit, empty list and unknown worker are all rejected before any task starts (`run_count() == 0`) | unit | `cargo test -p paladin-battalion --lib engine::graph && cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-05-03 | 05 | 4 | CF-03 | T-23-20 | `{muster.payload}` template namespace resolves; aggregation order stable across 20 shuffled runs | unit | `cargo test -p paladin-battalion --lib engine::input_mapping && cargo test -p paladin-battalion --lib engine::superstep::tests::task_key_order_is_stable_across_twenty_shuffled_runs` | ✅ | ✅ green (13 + 1) |
| 23-06-D | 06 | 5 | CF-03 | — | D-14 payload-contract freeze (`checkpoint:decision`, gate `blocking`) — auto-selected option-a under `_auto_chain_active`, recorded in 23-06-SUMMARY | decision | N/A | — | ⚡ resolved |
| 23-06-01 | 06 | 5 | CF-03 | T-23-23, T-23-24, T-23-26 | Resume mid-muster re-runs exactly the unfinished tasks; resumed final Battlefield equals the uninterrupted run; progress Waypoint holds the superstep-start snapshot (no double merge) | unit | `cargo test -p paladin-battalion --lib engine::superstep && cargo test -p paladin-ai-core --lib waypoint` | ✅ | ✅ green (29 waypoint) |
| 23-06-02 | 06 | 5 | CF-03 | T-23-25, T-23-27 | Additive `muster_progress` field round-trips on all three Waypoint backends (`#[serde(default)]`, `None` stays `None`) | contract | `cargo test -p paladin-storage --lib --all-features -- muster_progress checkpoint_ns` *(corrected)* | ✅ | ⚠️ partial — 12 selected: SQLite 4 + in-memory 4 green; Postgres 4 self-skip (`SKIP: postgres-test not reachable`) |
| 23-07-01 | 07 | 5 | CF-03 | T-23-28, T-23-29 | `EngineConfig::validate` rejects zero limits; `EngineConfig::default()` matches the engine's built-in defaults; `APP_ENGINE_MAX_MUSTER_TASKS` reaches the effective limits | unit | `cargo test -p paladin-ai --lib config::engine` | ✅ | ✅ green (7) |
| 23-07-02 | 07 | 5 | CF-03 | T-23-30 | §9.5 `EngineConfig` claim closed; CF-05 no-config decision recorded | doc-smoke | `! grep -q 'not yet in the tree' MIGRATION.md && grep -q 'APP_ENGINE_MAX_MUSTER_TASKS' MIGRATION.md` *(corrected)* | ✅ | ✅ green |
| 23-08-01 | 08 | 6 | CF-04 | T-23-31, T-23-34, T-23-35 | Unmapped child fields stay private; mapped outputs return as the parent node's delta; cancellation observed at the child superstep boundary | unit | `cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-08-02 | 08 | 6 | CF-04 | T-23-32, T-23-33 | Direct and transitive recursive embedding rejected at validation; child graph validated with the parent's registries; deep-but-acyclic nesting validates | unit | `cargo test -p paladin-battalion --lib engine::graph` | ✅ | ✅ green |
| 23-09-D | 09 | 7 | CF-04 | — | D-20 child-thread derivation (`checkpoint:decision`, gate `blocking`) — auto-selected option-a, recorded in 23-09-SUMMARY | decision | N/A | — | ⚡ resolved |
| 23-09-01 | 09 | 7 | CF-04 | T-23-36, T-23-37, T-23-38, T-23-39, T-23-40 | `ThreadId::child` is injective under adversarial names and bounded at 256 chars; `latest(child_thread)` returns the child's own Waypoint; resume-mid-child resumes where the child stopped; `restart_on_resume` abandons, never deletes | unit | `cargo test -p paladin-ai-core --lib waypoint && cargo test -p paladin-battalion --lib engine::superstep` | ✅ | ✅ green |
| 23-09-02 | 09 | 7 | CF-04 | T-23-37 | `checkpoint_ns` round-trips on all three backends; Formation / Phalanx / Campaign bridges embed inside a branching Campaign, kill-and-resume after the child's first superstep repeats no child work | integration + contract | `cargo test --test subgraph_formation_in_campaign` and the 23-06-02 storage command | ✅ | ✅ green (29) / ⚠️ Postgres tier self-skip as 23-06-02 |
| 23-10-D | 10 | 7 | CF-02, CF-03, CF-04 | — | D-18 fingerprint v3 (`checkpoint:decision`, gate `blocking`) — auto-selected option-a, recorded in 23-10-SUMMARY | decision | N/A | — | ⚡ resolved |
| 23-10-01 | 10 | 7 | CF-02, CF-03, CF-04 | T-23-41, T-23-42, T-23-43, T-23-44 | Worker templates, Battalion children, `StateMap` entries and directive parsers all move the fingerprint; version tag is `v3`; `EngineLimits` (incl. `max_muster_tasks`) stay excluded | unit | `cargo test -p paladin-battalion --lib engine::graph` | ✅ | ✅ green |
| 23-11-01 | 11 | 8 | CF-03 | T-23-47 | E2E-3 muster / defer / ordering half passes end to end; the scripted recovering worker is delimited as a Phase 25 seam | integration | `cargo test --test e2e_muster_defer_order` | ✅ | ✅ green (30) |
| 23-11-02 | 11 | 8 | CF-03 | T-23-45, T-23-46 | 50-task muster completes under the multi-thread runtime with exact counts and ordering; deterministic across repeats; timeout-guarded, no `#[ignore]` | unit (multi_thread) | `cargo test -p paladin-battalion --lib engine::superstep::tests::fifty_task_muster_runs_to_completion_under_multi_thread` | ✅ | ✅ green |
| 23-12-01 | 12 | 9 | CF-01, CF-02, CF-03, CF-04, CF-05 | T-23-48 | Control-flow guide exists, is linked once from the book, and carries the LLM egress-boundary paragraph | doc-smoke | `grep -c 'control-flow.md' docs/src/SUMMARY.md \| grep -qx 1 && cargo doc --workspace --no-deps` | ✅ | ✅ green (`cargo doc` exit 0; 17 `unresolved link` warnings — see audit trail) |
| 23-12-02 | 12 | 9 | CF-01, CF-02, CF-03, CF-04, CF-05 | T-23-49 | §9.2 register carries no `TBD — owner CF-` rows; CHANGELOG `[Unreleased]` names M-B-01 and CF-02…05 | doc-smoke | `! grep -q 'TBD — owner CF-' MIGRATION.md && grep -q 'M-B-01' CHANGELOG.md` *(corrected)* | ✅ | ✅ green |
| 23-12-03 | 12 | 9 | CF-01, CF-02, CF-03, CF-04, CF-05 | T-23-50, T-23-51 | Program gates clean with no new advisory suppression | gate | `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && make security` | ✅ | ✅ green (fmt + clippy re-run 2026-09-04; `make security` exit 0 recorded by the 23-SECURITY.md audit the same day, not re-run here) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky/partial · ⚡ auto-resolved decision gate*

Additional regression evidence re-run 2026-09-04: `cargo test --test e2e_crash_resume` (27 green, Phase 22 contract), `cargo test --test war_engine_tracer` (3 green), `! grep -rn 'defaulting to true' crates/` (exit 0 — the BUG-01 branch is gone).

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Every task with a behavioral requirement has a green test in the tree; this audit generated no new test files.

- [x] `crates/paladin-battalion/src/edge_evaluator.rs` — 3 unit tests (CF-01)
- [x] `crates/paladin-battalion/src/engine/directive_parser.rs` — 10 unit tests (CF-02)
- [x] `crates/paladin-battalion/src/llm_decision.rs` — 9 unit tests (CF-05)
- [x] `crates/paladin-battalion/src/engine/input_mapping.rs` — 13 unit tests (CF-03)
- [x] `crates/paladin-storage/src/waypoint/contract_tests.rs` — 4 shared contract fns, wrapped per backend in `sqlite.rs`, `in_memory.rs`, `postgres.rs` (CF-03, CF-04)
- [x] `src/config/engine.rs` — 7 unit tests (CF-03)
- [x] `tests/integration/subgraph_formation_in_campaign_test.rs` — 29 tests (CF-04)
- [x] `tests/integration/e2e_muster_defer_order_test.rs` — 30 tests (CF-03)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Postgres Tier-2 Waypoint contract suite: `muster_progress_round_trips`, `muster_progress_none_round_trips_as_none`, `checkpoint_ns_round_trips`, `checkpoint_ns_none_round_trips` in `crates/paladin-storage/src/waypoint/postgres.rs` | CF-03, CF-04 | Docker is unavailable in this devcontainer, so `postgres_reachable()` returns false and each test body prints `SKIP: postgres-test not reachable` and returns `ok` with zero assertions. The branch is 99 commits ahead of `origin/feature/phase-22`; the last green CI run (`4885380c`, job "Postgres Waypoint Contract Suite (live server)": success) predates `98e0a405`, the commit that added these tests, so no CI run has exercised them yet. | Push the branch and confirm the `postgres-integration` job (`.github/workflows/ci.yml:817`) is green on the head SHA; or locally `docker compose -f docker/docker-compose.test.yml up -d postgres-test` then `WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test cargo test -p paladin-storage --features postgres --lib waypoint::postgres -- --test-threads=1 --nocapture` (the `make test-integration-docker` recipe) and check that no `SKIP:` line is printed. Tracked as UAT item 1. |
| No config / env / feature / builder option restores BUG-01's always-true edge behavior | CF-01 | Judgment-tier prohibition (a universal negative); the project's policy requires human sign-off rather than an LLM-judge pass | Positive evidence, exit 0 on 2026-09-04: `! grep -rn 'defaulting to true' crates/` and `! grep -rnE 'APP_(LLM_DECISION\|ENGINE_[A-Z_]*(EDGE\|EVALUATOR\|SEMANTIC))\|APP_[A-Z_]*SEMANTIC' src/ crates/ config* docs/src`; the only `APP_ENGINE_*` variables are `MAX_MUSTER_TASKS`, `MAX_NODE_VISITS`, `MAX_SUPERSTEPS`, `RUN_TIMEOUT_SECS`, `WAYPOINT_DURABILITY`. Sign off via UAT item 2. |
| LLM prompt, response or credential is never interpolated into an error, log or trace | CF-05 | Judgment-tier prohibition | `cargo test -p paladin-battalion --lib llm_decision commander` — the exclusion assertions live at `llm_decision.rs:451`, `:491` and `commander.rs:2326`; `llm_error_class()` returns a fixed `&'static str`. Sign off via UAT item 2. |
| Semantic / `LlmDecision` routing is unreachable without explicit in-code configuration | CF-05 | Judgment-tier prohibition | `cargo test -p paladin-battalion --lib default_strategy_selection_is_heuristic_and_unchanged` plus the toggle grep above; no evaluator is registered by default (`campaign_service.rs:66`). Sign off via UAT item 2. |
| Unmapped child Battlefield fields never leak to the parent or its Waypoint | CF-04 | Judgment-tier prohibition | `cargo test -p paladin-battalion --lib unmapped_child_fields_stay_private` (green 2026-09-04). Sign off via UAT item 2. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (3 decision gates are N/A by design)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none were MISSING)
- [x] No watch-mode flags
- [x] Feedback latency < 30s warm for the quick command
- [ ] `nyquist_compliant: true` set in frontmatter — blocked only on the Postgres tier: flip once the `postgres-integration` CI job is green on a SHA at or after `98e0a405`, then re-run `/gsd-validate-phase 23`

**Approval:** partial — validated 2026-09-04 (24 of 24 automated task rows green; 1 backend tier CI-gated; 4 judgment-tier prohibitions awaiting UAT sign-off)

---

## Validation Audit 2026-09-04

| Metric | Count |
|--------|-------|
| Gaps found | 5 |
| Resolved | 4 |
| Escalated | 1 |

**Gate:** the gap plan was auto-approved as "Fix all gaps" because `.planning/config.json` sets `workflow._auto_chain_active: true` (the same carve-out under which this phase's three `blocking` decision checkpoints were auto-selected). No human confirmed the gate interactively.

**Auditor not spawned:** none of the five gaps required generating a test — four were `no_automated_command` defects in the PLANs' recorded commands (fixed by correcting the map, each corrected command verified exit 0 live) and one is an environment gap the auditor would only have escalated. Spawning `gsd-nyquist-auditor` would have produced no artifact, so the orchestrator resolved the map directly.

| # | Task | Gap | Type | Resolution |
|---|------|-----|------|------------|
| 1 | 23-06-02 | PLAN command `cargo test -p paladin-storage --lib waypoint::contract_tests` selects **0 tests** (the contract fns are `pub async fn` helpers, not `#[test]`s; the per-backend wrappers live under `waypoint::{sqlite,in_memory,postgres}::tests`), so it exited 0 without running anything | no_automated_command | Command corrected to `-- muster_progress checkpoint_ns`, which selects the 12 wrapper tests (verified live: 12 passed) |
| 2 | 23-01-02 | `grep -c 'TBD — owner CF-01' MIGRATION.md` returns 0 and exits 1 in the desired state, so the chained PLAN command fails on success | no_automated_command | Rewritten as `! grep -q …`; exit 0 verified |
| 3 | 23-07-02 | Same inversion for `grep -c 'not yet in the tree' MIGRATION.md` | no_automated_command | Rewritten as `! grep -q …` plus the positive `APP_ENGINE_MAX_MUSTER_TASKS` check; exit 0 verified |
| 4 | 23-12-02 | Same inversion for `grep -c 'TBD — owner CF-' MIGRATION.md` | no_automated_command | Rewritten as `! grep -q …` plus the `M-B-01` CHANGELOG check; exit 0 verified |
| 5 | 23-06-02 / 23-09-02 (Postgres backend) | The 4 Postgres contract tests report `ok` while executing zero assertions (self-skip, no Docker); no CI run yet covers commits ≥ `98e0a405` | PARTIAL (environment) | Escalated to Manual-Only (CI-gated) and UAT item 1; `nyquist_compliant` stays `false` until that job is green |

**Observation (not a Nyquist gap):** `cargo doc -p paladin-battalion -p paladin-ai-core --no-deps` emits 17 `unresolved link` warnings at `a45d8514` versus 13 at the phase base `38abd07f` (measured in a detached worktree). At least `directive.rs:3` (`StateNode::run`), `input_mapping.rs:30` (`llm_error_class`), `engine/mod.rs:624` (`Waypoint`) and `graph.rs:479` (`hooks`) are in files this phase created or edited. 23-12-SUMMARY's "all pre-existing" claim is true of plan 23-12's docs-only diff but not of the phase as a whole. `cargo doc` still exits 0 and no gate enforces `rustdoc::broken_intra_doc_links`; recommended as a small doc follow-up, not a blocker.
