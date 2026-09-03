# 08 — Traceability Matrix (Gap Analysis → Specification Requirements)

**Purpose:** Confirms that every item from the prior competitive gap analysis (IDs G-01…G-29, BUG-01) is covered by at least one requirement in PRDs 01–07. This document is the checklist for the post-implementation verification pass. The implementation agent may use it for coverage self-checks but should treat the PRDs themselves as the source of truth for behavior.

| Gap | Topic | Covering requirements | Notes |
|---|---|---|---|
| G-01 | Cyclic graph execution | ENG-FR-02, ENG-FR-03; CF-FR-07 (Goto loops) | Bounded by max_supersteps / max_node_visits |
| G-02 | Typed shared state + reducers | ENG §3.1, ENG-FR-05, 07–10 | Battlefield + DispatchRule |
| G-03 | Per-step checkpointing / durable execution | ENG-FR-11, 12, 15–17; PLAT-FR-03 | Waypoint per superstep; resume; worker redelivery |
| G-04 | Thread/checkpoint addressing | ENG §3.3, ENG-FR-13, 14 | ThreadId + WaypointId + fingerprint |
| G-05 | Human-in-the-loop pause/resume | HITL-FR-01…06; PLAT-FR-06 | Parley, Gate node, resume API |
| G-06 | History / replay / fork ("time travel") | HITL-FR-07…12; PLAT threads/fork endpoints | Immutable chronicle, fork_of lineage |
| G-07 | Dynamic routing from node output | CF-FR-05…08 | Directive / NextStep::Goto/End |
| G-08 | Dynamic parallel fan-out (map-reduce) | CF-FR-09…13; FT-FR-06 (retry per task) | Muster + task_key ordering |
| — (deferred/join precision) | Join & defer semantics | ENG-FR-06; CF-FR-12 | Not-firing edges don't deadlock joins |
| G-09 | Subgraph composition | CF-FR-14…17; HITL-FR-12 (fork interaction) | Battalion-as-node, namespaced waypoints |
| G-10 | Per-node retry policy | FT-FR-03…07 + FT §2.1 | Backoff, jitter, predicate, attempt isolation |
| G-11 | Per-node timeout (wall + idle) | FT-FR-08…10 | heartbeat-based progress |
| G-12 | Typed error handlers / compensation | FT-FR-01, 02, 11…15 | Transience taxonomy + Route/Absorb/Custom |
| G-13 | Model fallback | FT-FR-16, 17; RT-FR-09 | Adapter + middleware exposure |
| G-14 | Node result caching | FT-FR-18…20 | CachePolicy + NodeCachePort |
| G-15 | Graceful shutdown | ENG-FR-23; HITL-FR-13…15; PLAT-FR-04 | Cooperative halt, SIGTERM wiring, remote cancel |
| G-16 | Middleware pipeline | RT-FR-01…09; ENG-FR-22 | Two layers documented (node vs. in-loop) |
| G-17 | Context-window management | RT-FR-10…12 | TokenCounterPort, trim, summarize |
| G-18 | Cross-session long-term memory | RT-FR-13…16 | VaultPort + confinement + recall |
| G-19 | Structured output | RT-FR-17…19 | Schema-constrained + repair + engine field write |
| G-20 | LLM-driven routing / strategy selection | CF-FR-18, 19 | LlmDecision condition + Commander semantic mode |
| G-21 | Prebuilt agents & tool-error ergonomics | RT-FR-23, 24 | reasoning_agent preset, tool-error feedback |
| G-22 | Provider breadth | RT-FR-20…22 | OpenAI-compatible generic, Gemini, local recipe |
| G-23 | Background runs + task queue | PLAT-FR-01…07 + PLAT §2.1–2.2 | Queue port, worker pool, streaming |
| G-24 | Cron + webhooks as API | PLAT-FR-13…15 | incl. SSRF guard |
| G-25 | Versioned assistants | PLAT-FR-08…12 | Immutable versions, freeze-at-submit, WarGraphDoc |
| G-26 | Threads API | HITL-FR-16; PLAT §2.1 thread endpoints | |
| G-27 | Visual debugging | OBS-FR-08…10 | Mermaid/DOT export + execution overlay + inspector page |
| G-28 | Trace/observability + eval | OBS-FR-01…07 (trace/OTel), OBS-FR-11…15 (eval) | |
| G-29 | Multi-language client access | PLAT-FR-17 | Generated-client CI gate (hand-written SDKs out of scope) |
| BUG-01 | Custom edge condition silently true | CF-FR-01…04; overview §7 | Fail-closed at validation; test-first mandate — fixed CF-01, Plan 23-01: RED `b2d05045` (`test(23-01): reproduce BUG-01 on both custom-edge-condition paths (red)`), GREEN `8d5ef333` (`fix(23-01): fail closed on unregistered custom edge conditions (green)`); grep-absence confirmed, `grep -rn "defaulting to true" crates/` returns no matches |
| BUG-02 | Silent stranded node (unreachable node, run reports Completed) | ENG-FR-02a; doc 01 acceptance 2a; overview §7 | Reachability-from-entry validation; worker_template / Route targets / `dynamic_target` exemptions; test-first; pre-release fix, no migration entry |
| BUG-03 | Cycle-bootstrap starvation (non-entry cycle node fed from outside can never take its first turn, run reports Completed) | ENG-FR-06a; overview §7 | Starvation-release fallback pass + validate-time guard + run-end truthful-outcome check; test-first; pre-release fix, no migration entry |
| BUG-04 | resume rebuilt the Frontier from scratch, losing pre-crash edge resolutions; a pending join across the crash point is dropped and the run reports Completed | ENG-FR-12a; overview §7 | frontier snapshot persisted on the Waypoint, keyed by edge identity, seeded on resume; shared contract-suite round-trip and pre-BUG-04 compatibility cases on all three backends; test-first; pre-release fix, no migration entry |

**Verification protocol (for the post-implementation audit):**
1. For each row, locate the implementing code + the tests named in the PRD's Test Plan; confirm the acceptance criteria of the owning PRD pass in CI.
2. Confirm cross-cutting X-01…X-09 per epic (spot-check dependency directions with `cargo tree` / import review; coverage report ≥ 82%; clippy clean).
3. Run the three program E2E scenarios (overview §6) and the eval-scenario dogfood copies (OBS-FR-15).
4. Confirm BUG-01's old code path is absent (grep for the warn-and-default-true branch) and the fix landed test-first. Confirm BUG-02's fix: `WarGraph::validate()` rejects a stranded self-loop-only node (run the regression test), the fix landed test-first, and no test fixture still works around strandedness by artificially wiring stranded nodes to entry.
5. File any FR without a passing test, any test without an FR ("orphan behavior"), and any deviation from the ubiquitous-language names in §4 of the overview as findings.
6. **Compatibility audit (X-03/X-10):** diff the public API of every publishable crate against v0.9.0 (`cargo semver-checks` output plus a manual `cargo public-api`-style diff). Every enum-variant, struct-field, or trait-method change to a pre-existing type must have a matching row in `MIGRATION.md` §9.2 with a stated mitigation; every semver-checks allowlist entry must correspond to a "deliberate-breaking: Y" row; any unregistered change is a finding. Confirm no pre-existing public trait gained a required method.
7. **Behavioral-change audit:** confirm `MIGRATION.md` §9.1 contains M-B-01 (BUG-01), M-B-02 (shutdown grace), M-B-03 (tool-error default, with the chosen default stated) and nothing else — or that any additional entry was raised as a stop-and-flag item with a recorded decision.
8. **Toolchain audit (X-11):** MSRV job present and green on the MSRV toolchain with `--all-features`; `rust-version`, README badge and `MIGRATION.md` §9.3 agree; `cargo tree -e features` for default features shows no new heavyweight dependencies; `cargo deny`/`cargo audit` green.
9. **Config-compat test:** the integration test required by §9.5 (boot v0.10 with a v0.9 sample config → legacy behavior) exists and passes; §9.6's `openapi.json` golden diff for pre-existing paths is empty.
10. **Release readiness:** all crates at `0.10.0`; changelogs updated; `cargo publish --dry-run` succeeds in dependency order; `MIGRATION.md` contains no "TBD".
