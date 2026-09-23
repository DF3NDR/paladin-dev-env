---
phase: 23
slug: control-flow-dynamic-routing-fan-out-subgraphs
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-09-04
register_authored_at_plan_time: true
---

# Phase 23 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase goal: nodes steer their own routing at runtime (`Directive` / `NextStep`), fan out into
map-reduce `Muster` workers, nest Battalions as subgraphs, optionally route by LLM evaluation, and
close the BUG-01 custom-edge-condition defect fail-closed.

All twelve PLANs (23-01 through 23-12) carried a `<threat_model>` block, so this register was
authored at plan time and this audit **verifies** those mitigations rather than constructing a
register retroactively. Verification depth is ASVS L1 (grep-level: the cited mitigation is present
in the cited file), per the short-circuit rule in the secure-phase workflow.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| workflow author -> `EdgeConditionEvaluator` impl (23-01) | Third-party in-process code registered by name and invoked inside the scheduler; its `Err` and its verdict both steer routing. | Routing verdicts; evaluator errors (name + structured reason, never Battlefield content) |
| caller -> `WarGraph::validate` / `CampaignExecutionService::execute` (23-01) | The validation verdict is the trust surface: an accepted graph that then routes on an unevaluated condition is the violation CF-01 closes. | Graph acceptance verdict |
| node implementation -> scheduler (23-02, 23-04, 23-05) | A `StateNode` now returns routing instructions (`Goto`, `Muster`, `End`, `Parley`), possibly parsed from model output; every runtime-produced value is validated at Directive receipt before the scheduler acts on it. | `NodeId` targets, muster task lists, per-task `serde_json::Value` payloads |
| caller -> `RunOutcome` / `WaypointStatus` (23-02) | The completion verdict is the trust surface; a `Completed` that hides an un-run node is the violation 22.1 closed and this phase must not reopen. | Run status |
| process -> third-party LLM API (23-03) | The rendered `prompt_template` leaves the process. This is the phase's only outward data flow, and the author controls what it contains. | Live Battlefield state or the source node's output (author-controlled, may be sensitive) |
| model response -> routing decision (23-03, 23-04) | An untrusted model answer selects an edge, a Battalion strategy, or is parsed into a `StructuredDirective`; it is matched against closed author-declared lists or the schema allowlist, never executed or interpolated. | Model text; envelope field names and values |
| task payload -> worker execution; worker task -> shared Battlefield (23-05) | A per-task payload reaches its own worker and no other and never enters shared state; sibling deltas merge into one state object in a total order. | Muster payloads; `StateDelta`s |
| engine -> `WaypointPort` backend; stored Waypoint -> resumed run (23-06, 23-09, 23-10) | Progress Waypoints (including worker deltas), derived child thread ids, `checkpoint_ns`, and the `v3` graph fingerprint are persisted and trusted on resume. | Battlefield snapshots (may contain model output); `MusterProgress`; `ThreadId`s; fingerprint digest |
| process environment / config file -> `EngineConfig` (23-07) | `APP_ENGINE_*` variables and the `engine:` section set execution and resource limits through the single `validate()` gate. | Numeric limits and a durability enum only |
| parent Battlefield <-> child Battlefield; parent resources -> child run; graph author -> nesting depth (23-08) | The `StateMap` is the complete declared channel between two state namespaces; the child inherits ports, registries and the cancellation token; an `Arc<WarGraph>` chain sets recursion depth. | Mapped fields only; shared `Arc` ports and registries; nesting depth |
| (parent thread, node id) -> derived child `ThreadId`; child Waypoints -> shared backend (23-09) | An author-controlled `NodeId` with no charset restriction is combined with a thread id to address durable storage; the mapping must be injective. | `NodeId` + parent `ThreadId` |
| graph author -> hashed byte stream (23-10) | Author-supplied `NodeId`s, `FieldName`s and `StateMap` entries are concatenated into the fingerprint input; the encoding must be unambiguous. | Author strings (length-prefixed) |
| test harness -> engine under real concurrency (23-11) | The only place the muster path is exercised under genuine multi-thread contention. | None (in-process, mocks) |
| documentation -> workflow author; dependency graph -> `make security`; public API -> downstream crates (23-12) | The guide names the egress boundary; cargo-audit and cargo-deny gate advisories and licences; `cargo semver-checks` detects unregistered breaking changes. | Docs; RustSec advisory DB; public API surface |

---

## Threat Register

Evidence paths are relative to the repository root; `superstep.rs`, `graph.rs`, `mod.rs`,
`directive_parser.rs` are under `crates/paladin-battalion/src/engine/`, `llm_decision.rs`,
`commander.rs`, `edge_evaluator.rs`, `campaign_service.rs` under `crates/paladin-battalion/src/`,
`waypoint.rs` under `crates/paladin-core/src/platform/container/`, and `engine.rs` under
`src/config/`. Line numbers are as of commit 091ed6e7.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-23-01 | Tampering (routing contract) | `evaluate_edge_condition` on both paths | high | mitigate | `EngineError::UnregisteredEdgeCondition` (`mod.rs:370`) raised by `WarGraph::validate` (`graph.rs:823`); one shared registry in `edge_evaluator.rs` consumed by both `campaign_service.rs` and the engine; red-then-green commits b2d05045 -> 8d5ef333. | closed |
| T-23-02 | Denial of Service | third-party `EdgeConditionEvaluator::evaluate` awaited in the superstep loop | medium | accept | R-23-01. `EngineLimits::max_supersteps` (default 50, `graph.rs:264`) remains the run-level bound; per-node timeouts are Phase 25 (FT-03). | closed |
| T-23-03 | Information Disclosure | `EdgeEvaluatorFailed` fields | medium | mitigate | Variant carries exactly `from`, `to`, `evaluator`, `source` (`mod.rs:381-389`); no Battlefield JSON or evaluated output string. | closed |
| T-23-04 | Repudiation | git history of the BUG-01 fix | low | mitigate | Two-commit order in history: b2d05045 `test(23-01)` precedes 8d5ef333 `fix(23-01)`. | closed |
| T-23-05 | Denial of Service | `NextStep::Goto` routing loops | high | mitigate | `unbounded_goto_loop_trips_the_node_visit_limit` (`superstep.rs:3087`); `max_node_visits` default 25 (`graph.rs:265`). | closed |
| T-23-06 | Spoofing (node routing authority) | Goto target validation | high | mitigate | `EngineError::GotoUnknownNode` raised at Directive receipt (`superstep.rs:1276`); Goto-only targets must be declared via `WarGraph::mark_dynamic_target` (`graph.rs:281`). | closed |
| T-23-07 | Tampering (truthful-outcome contract) | `End`'s suppression of `StarvedNodeAtCompletion` | high | mitigate | Suppression gated on `end_requested` alone (`superstep.rs:1255-1317`); `starvation_completion_check_still_fires_when_no_node_ended_the_run` (`superstep.rs:3269`). | closed |
| T-23-08 | Repudiation | `NextStep::Parley` in this phase | low | mitigate | Typed `EngineError::ParleyNotSupported { node }` (`mod.rs:414`, raised `superstep.rs:1330`). | closed |
| T-23-09 | Information Disclosure | `LlmDecisionEvaluator` rendered prompt (egress) | high | mitigate | Module rustdoc "Security: the rendered prompt is an egress boundary" (`llm_decision.rs:24`); errors carry a fixed `llm_error_class` `&'static str` (`llm_decision.rs:306`); tests assert the error message excludes prompt and choice text (`llm_decision.rs:451`, `:491`). | closed |
| T-23-10 | Information Disclosure | `StrategySelection` `Debug` and Commander logging | high | mitigate | Manual `impl Debug for StrategySelection` prints a `<dyn LlmPort>` placeholder and the model string only (`commander.rs:71-79`); fallback reason is `llm_error_class` (`commander.rs:1121`); `commander.rs:2326` asserts the reason excludes the response body. | closed |
| T-23-11 | Tampering (routing by model answer) | choice matching and catalog parsing | medium | mitigate | `trim()` plus `eq_ignore_ascii_case` against the declared list (`llm_decision.rs:269-271`, `commander.rs:1066-1069`); unmatched answers resolve through `on_ambiguous` (default `OnAmbiguous::Fail`, `llm_decision.rs:146`). | closed |
| T-23-12 | Denial of Service / cost | one LLM call per outgoing edge | medium | mitigate | `one_llm_call_per_decision_per_superstep` (`llm_decision.rs:372`). | closed |
| T-23-13 | Elevation of Privilege (unrequested paid calls) | default configuration | high | mitigate | `grep -rn` for `APP_LLM_DECISION` or `APP_.*SEMANTIC` across `crates/`, `config*`, `docs/` returns no matches; `StrategySelection::default()` is `Heuristic` (`commander.rs:46`, `:53`); no evaluator registered by default (`campaign_service.rs:66`). | closed |
| T-23-14 | Tampering (Battlefield via model output) | `StructuredDirective` envelope delta | high | mitigate | `envelope_delta_naming_an_unknown_field_fails_the_run` (`superstep.rs:2719`). | closed |
| T-23-15 | Spoofing (routing authority by a model) | `StructuredDirective.next` | high | mitigate | A parsed Goto passes the same `GotoUnknownNode` check at Directive receipt (`superstep.rs:1276`) and the `mark_dynamic_target` declaration rule. | closed |
| T-23-16 | Denial of Service | JSON parsing of arbitrary model output | medium | mitigate | `#[serde(deny_unknown_fields)]` on `Envelope` (`directive_parser.rs:89`); typed serde deserialization, no hand-rolled scanner. | closed |
| T-23-17 | Tampering (silent behaviour change on upgrade) | `DirectiveParser` default | high | mitigate | `DirectiveParser::default() == PlainOutput` asserted (`directive_parser.rs:255`); `plain_output_directive` reproduces the `output_field` write (`directive_parser.rs:161`, `:183`). | closed |
| T-23-18 | Denial of Service | unbounded muster fan-out | high | mitigate | `max_muster_tasks` default 100 (`graph.rs:267`); widening `u32 -> usize` compare in `muster_task_count_exceeds_limit` before any dispatch (`superstep.rs:555-573`); `muster_exceeding_the_limit_fails_before_any_task_starts` (`superstep.rs:3823`). | closed |
| T-23-19 | Information Disclosure | one worker observing another's payload | high | mitigate | `each_worker_sees_only_its_own_payload` (`superstep.rs:3558`); `muster_payload_never_enters_the_battlefield` (`superstep.rs:3623`). | closed |
| T-23-20 | Tampering (aggregated results) | worker delta merge order | high | mitigate | `EngineError::DuplicateMusterTaskKey` (`mod.rs:448`, raised `superstep.rs:580`); `tasks.sort_by(task_key)` (`superstep.rs:605`); stable `deltas.sort_by(NodeId)` merge (`superstep.rs:1500-1503`). | closed |
| T-23-21 | Tampering (partial dispatch on rejection) | validate-before-dispatch ordering | medium | mitigate | All muster checks run at Directive receipt; rejection tests assert `run_count() == 0` on worker fixtures (`superstep.rs:3819`, `:3869`, `:4424`). | closed |
| T-23-22 | Spoofing (routing work to an arbitrary node) | `MusterTask.worker` | medium | mitigate | `EngineError::MusterUnknownWorker` (`mod.rs:476`); `graph.is_worker_template(&task.worker)` gate (`mod.rs:915`); `worker_templates: HashSet<NodeId>` (`graph.rs:305`). | closed |
| T-23-23 | Tampering (resumed run's completeness) | `muster_progress`-driven resume | high | mitigate | `resume_mid_muster_runs_exactly_the_unfinished_tasks` (`superstep.rs:4089`); `resumed_muster_final_battlefield_equals_the_uninterrupted_run` (`superstep.rs:4159`). | closed |
| T-23-24 | Tampering (double-merged state) | incremental merge | high | mitigate | `progress_waypoint_battlefield_equals_the_superstep_start_snapshot` (`superstep.rs:3985`). | closed |
| T-23-25 | Information Disclosure | progress Waypoints persisting worker deltas | medium | accept | R-23-02. `WaypointRetentionConfig` is the existing control; MIGRATION.md M-B-04 already documents that Waypoint snapshots may contain raw prompts and outputs. | closed |
| T-23-26 | Denial of Service | unbounded progress-Waypoint writes | medium | mitigate | One write per completed task, bounded by construction by the `max_muster_tasks` enforcement that runs before dispatch (T-23-18 evidence). | closed |
| T-23-27 | Repudiation | ENG-FR-11 "one Waypoint per superstep" wording | low | mitigate | Clarification note landed in commit 98e0a405 (`.project/v0.10.0/01-battlefield-state-and-execution-engine.md:218` and `08-traceability-matrix.md:16`); `waypoint.rs:533` documents the mid-muster progress Waypoint. | closed |
| T-23-28 | Denial of Service | operator sets a limit to zero or an absurd value | medium | mitigate | `EngineConfig::validate` rejects zero for all three counts and `Some(0)` for the timeout (`engine.rs:99-109`); `validate_rejects_zero_limits` (`engine.rs:237`). | closed |
| T-23-29 | Tampering (silent behaviour change on upgrade) | `EngineConfig::default()` | high | mitigate | `default_engine_config_matches_todays_engine_defaults` (`engine.rs:228`). | closed |
| T-23-30 | Information Disclosure | config values in logs or `Debug` output | low | accept | R-23-03. `EngineConfig` carries numeric limits and a durability enum only. | closed |
| T-23-31 | Information Disclosure | unmapped child fields reaching the parent or its Waypoint | high | mitigate | `unmapped_child_fields_stay_private` (`superstep.rs:6717`); `NodeSpec::Battalion` rustdoc states only `StateMap` fields cross (`graph.rs:70-76`). | closed |
| T-23-32 | Denial of Service | recursive or unbounded subgraph nesting | high | mitigate | `EngineError::RecursiveEmbedding` from a path-set walk over child fingerprints at validation (`graph.rs:647-673`); `deep_but_acyclic_nesting_validates` (`graph.rs:3321`). | closed |
| T-23-33 | Elevation of Privilege (child escaping fail-closed contract) | child validation registries | high | mitigate | `child_graph_is_validated_with_the_parents_registries` (`graph.rs:3135`). | closed |
| T-23-34 | Tampering (parent state via child output) | mapped outputs merged into the parent | medium | mitigate | `state_map_outputs_return_as_the_parent_nodes_delta` (`superstep.rs:6651`). | closed |
| T-23-35 | Denial of Service | a child ignoring cancellation | medium | mitigate | `cancellation_is_observed_at_the_child_superstep_boundary` (`superstep.rs:7099`). | closed |
| T-23-36 | Tampering (delimiter collision) | child-`ThreadId` derivation | high | mitigate | `ThreadId::child` (`waypoint.rs:144`); `child_thread_derivation_is_injective_under_adversarial_names` (`waypoint.rs:753`). | closed |
| T-23-37 | Information Disclosure | one workflow's child checkpoints under another's thread | high | mitigate | `latest_on_the_child_thread_returns_the_childs_own_waypoint` (`superstep.rs:7237`). | closed |
| T-23-38 | Tampering (resumed child's completeness) | resume-mid-child | high | mitigate | `resume_of_a_parent_mid_child_resumes_the_child_where_it_stopped` (`superstep.rs:7292`); `restart_on_resume_true_runs_the_child_fresh` (`superstep.rs:7400`). | closed |
| T-23-39 | Denial of Service | child-thread id growth under deep nesting | medium | mitigate | `THREAD_ID_MAX_LEN = 256` enforced in `ThreadId::new` (`waypoint.rs:30`, `:66`); `ThreadId::child` returns `Result<_, ThreadIdError>` rather than truncating (`waypoint.rs:119-144`). | closed |
| T-23-40 | Repudiation | orphaned child chains under `restart_on_resume: true` | low | mitigate | Abandon-vs-overwrite policy documented in `superstep.rs:125` and `:303` ("deliberately ABANDONED, never deleted"); `restart_on_resume` field rustdoc (`graph.rs:77-82`). | closed |
| T-23-41 | Tampering (two graphs hashing identically) | the three new v3 fingerprint sections | high | mitigate | Every new section is written through `push_field` (`graph.rs:1159-1195`); difference tests cover worker templates, Battalion children, `StateMap` entries and directive parsers. | closed |
| T-23-42 | Tampering (silent reinterpretation of stored bytes) | the fingerprint version tag | high | mitigate | `fingerprint_version_tag_is_v3` (`graph.rs:1852`); golden literal pinned at `v3:a67a12f2...` (`graph.rs:1678`). | closed |
| T-23-43 | Denial of Service (spurious resume invalidation) | over-hashing an operator tunable | medium | mitigate | `fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits` (`graph.rs:1815`) keeps `EngineLimits` including `max_muster_tasks` excluded. | closed |
| T-23-44 | Repudiation | the golden digest re-pin | medium | mitigate | Re-pin rustdoc records that the fixture's construction is unchanged and exactly what moved the literal (`graph.rs:1666-1672`); regenerating commit 7751cb3d. | closed |
| T-23-45 | Tampering (aggregated results under real concurrency) | muster merge path at 50 tasks | high | mitigate | `fifty_task_muster_runs_to_completion_under_multi_thread` (`superstep.rs:7746`) and `fifty_task_muster_is_deterministic_across_repeats` (`superstep.rs:7802`), both `#[tokio::test(flavor = "multi_thread")]` with exact-count and ordering assertions. | closed |
| T-23-46 | Denial of Service (hung test masking a deadlock) | the stress test | medium | mitigate | `tokio::time::timeout` guard around each run (`superstep.rs:7752`, `:7810`); no `#[ignore]` attribute anywhere in `superstep.rs`. | closed |
| T-23-47 | Repudiation (stand-in mistaken for the real thing) | scripted recovering worker | medium | mitigate | Delimited "PHASE 25 SEAM (FT-FR-06)" block (`tests/integration/e2e_muster_defer_order_test.rs:18`, `:350-366`). | closed |
| T-23-48 | Information Disclosure | undocumented LLM egress in the control-flow guide | medium | mitigate | "Egress boundary" paragraph (`docs/src/user-guides/control-flow.md:227`). | closed |
| T-23-49 | Tampering (unregistered breaking API change) | the 9.2 register and semver allowlist | high | mitigate | `.cargo/semver-checks-allowlist.toml` has zero non-comment entries; 23-12-SUMMARY records `cargo semver-checks` vs v0.9.0 exit 0 for all 11 published crates. | closed |
| T-23-50 | Tampering (supply chain) | dependency advisories at phase close | high | mitigate | `make security` re-run live by this audit on 2026-09-04: exit 0, final line `advisories ok, bans ok, licenses ok, sources ok`; `git diff 38abd07f..HEAD -- .cargo/audit.toml deny.toml` is empty (no new suppression). | closed |
| T-23-51 | Repudiation (evidence reading as more assurance than it proves) | recorded gate evidence | high | mitigate | 23-12-SUMMARY records each gate's command, exit code and verbatim key output line (lines 170-239). | closed |
| T-23-SC (23-03) | Tampering | cargo installs | high | mitigate | The only dependency edit in the phase: `paladin-llm = { version = "0.9.0", path = "../paladin-llm" }` under `[dev-dependencies]` in `crates/paladin-battalion/Cargo.toml`; `cargo tree -e normal -p paladin-battalion` contains no `paladin-llm` entry. | closed |
| T-23-SC (23-01, 02, 04-12) | Tampering | cargo installs | high | accept | R-23-04. Phase-wide manifest diff (`git diff 38abd07f..HEAD`): `Cargo.lock` +1 line (the workspace-local `paladin-llm` entry above, not a registry fetch), root `Cargo.toml` +2 `[[test]]` targets (`subgraph_formation_in_campaign`, `e2e_muster_defer_order`), no other crate manifest touched. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-23-01 | T-23-02 | A hostile or hanging `EdgeConditionEvaluator` is author-supplied in-process code with the same trust as a `StateNode`. Per-node timeouts are Phase 25 (FT-03) scope; `EngineLimits::max_supersteps` remains the run-level bound. | Planner disposition in 23-01-PLAN `<threat_model>` (2026-09-03); recorded by /gsd-secure-phase | 2026-09-04 |
| R-23-02 | T-23-25 | Progress Waypoints persist worker deltas that may contain model output, exactly as superstep-complete Waypoints already may (MIGRATION.md M-B-04). They add write frequency, not a new class of stored content; `WaypointRetentionConfig` is the existing control. | Planner disposition in 23-06-PLAN `<threat_model>` (2026-09-03); recorded by /gsd-secure-phase | 2026-09-04 |
| R-23-03 | T-23-30 | `EngineConfig` carries only numeric limits and a durability enum, so its `Debug` output is not a disclosure surface. | Planner disposition in 23-07-PLAN `<threat_model>` (2026-09-03); recorded by /gsd-secure-phase | 2026-09-04 |
| R-23-04 | T-23-SC (23-01, 02, 04-12) | These eleven plans install no packages. The phase-wide manifest diff confirms the only dependency edit is Plan 23-03's workspace-local `paladin-llm` dev-dependency (mitigated above), plus two `[[test]]` targets in the root `Cargo.toml`. 23-RESEARCH.md "Package Legitimacy Audit" records zero external package additions. | Planner disposition in each PLAN `<threat_model>` (2026-09-03); confirmed by manifest diff and recorded by /gsd-secure-phase | 2026-09-04 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-04 | 53 | 53 | 0 | /gsd-secure-phase orchestrator (Claude Fable 5.1), ASVS L1 grep-depth |

### Audit notes (2026-09-04)

- **Input state B**: no prior SECURITY.md; 12 PLANs and 12 SUMMARYs present.
- **Register origin**: all 12 PLANs carried a parseable `<threat_model>`; register built from them
  (T-23-01 through T-23-51, plus T-23-SC declared once per plan and recorded here as two rows by
  disposition). No retroactive STRIDE construction was needed.
- **SUMMARY threat flags**: 23-06 and 23-08 declare `## Threat Flags: None`; the other ten
  SUMMARYs carry no `## Threat Flags` section. No unregistered attack surface was flagged by any
  executor.
- **Short-circuit applied**: `threats_open: 0` with a plan-time register at ASVS L1, so no
  `gsd-security-auditor` subagent was spawned. Every `mitigate` row above cites a concrete file
  and line or test name found by grep at commit 091ed6e7; every `accept` row is logged in the
  Accepted Risks Log.
- **Live re-checks performed by this audit** (beyond reading recorded evidence): `make security`
  (exit 0), `cargo tree -e normal -p paladin-battalion` (no `paladin-llm`), the `APP_LLM_DECISION`
  / `APP_.*SEMANTIC` grep (no matches), and the phase-wide manifest diff.
- **Not re-run here**: `cargo semver-checks` and the coverage gate; their evidence is the verbatim
  record in 23-12-SUMMARY (T-23-49, T-23-51). The Postgres Tier-2 contract tests for
  `muster_progress` / `checkpoint_ns` remain a UAT item (23-VERIFICATION.md, 23-UAT.md); they are
  a persistence round-trip check, not a threat mitigation, so they do not affect `threats_open`.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-04
