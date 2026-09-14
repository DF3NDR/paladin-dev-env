---
phase: 24
slug: pause-resume-history-graceful-shutdown
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-09-05
---

# Phase 24 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 24 (Pause / Resume / History / Graceful Shutdown) shipped 14 plans, every one of
which carried a `<threat_model>` block at plan time. This file consolidates those 14
registers, records the verification evidence for each mitigation, and logs the risks the
phase accepted by decision.

**Verification depth.** ASVS level 1 with `register_authored_at_plan_time: true`. The
preliminary grep-depth (L1) classification closed every entry, so per the secure-phase
short-circuit rule the deeper auditor pass was not spawned. Evidence below is a file, a
test name, or a document line that pins the mitigation; the test names listed in
"Verification Notes" were re-run live in this session.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| graph author / LLM output → `ParleyRequest.prompt`/`payload` | Author-templated or model-authored content is persisted verbatim into the Waypoint store and later read back by resume and by `GET /v1/threads/{id}/state` | Untrusted text; must never be enriched with engine-held credentials |
| caller → `WarEngine::resume_with(thread, responses)` | Caller-supplied `ParleyId`s and JSON values are delivered into a suspended workflow's state | Untrusted ids and values; scoped to one thread, validated per `ParleyKind` |
| expired parley → `on_expire: ResumeWithDefault(value)` | A pre-authored default stands in for a human decision | Author value; validated by the same per-kind validator as a real response |
| graph redeploy → stored `GraphFingerprint` | A changed graph must not be silently reinterpreted against an old suspended thread | Fingerprint compared before any state change |
| process A → `WaypointPort` → process B | A suspended thread's entire state crosses processes through the persisted Waypoint only | Serialized `Waypoint`; the only resumable record |
| branch identity → child thread id | A colliding derivation would let one branch read another branch's subgraph history | `ThreadId::child_on_branch` injective encoding |
| caller → `fork(thread, from, edit)` | A caller-supplied `StateDelta` is merged before a branch runs | Validated against the graph schema; mainline never mutated |
| OS signal (SIGTERM/SIGINT) → in-flight runs | An external signal interrupts mid-write work | Aborted deltas discarded; `Skipped` recorded; Halted Waypoint persisted |
| container orchestrator → process lifetime | kubelet SIGKILL deadline bounds how long the process may drain | `terminationGracePeriodSeconds` ≥ 2× grace |
| operator env → engine runtime | `APP_ENGINE_SHUTDOWN_GRACE_SECS` / `APP_ENGINE_GRACEFUL_SHUTDOWN` | Bounded by `EngineConfig::validate`; never enters the fingerprint |
| HTTP caller → `ParleyPort::resume_with` | Untrusted request data reaches the engine through this single method | Synchronous total validation before any spawn |
| thread fingerprint → `GraphRegistry` | The graph a continuation runs is selected by a stored value | Strict lookup; `GraphNotRegistered` with no fallback |
| config → database connection string | A Postgres URL must not be serialized into config, `Debug`, or logs | Config carries the env-var NAME only |
| unauthenticated internet → `/v1/threads/*` | Every thread route sits behind the same `require_authentication` layer as `/v1/agents/*`; resume additionally requires admin | Bearer credential; 401 / 403 |
| Waypoint content → HTTP response body | `state` returns Waypoint-derived content (raw prompts, model output) to any authenticated caller | Documented interim posture (D-24, PLAT-06 successor) |
| client → `limit` / `cursor` query params | Untrusted pagination reaches a storage query | `limit` capped at 100; unparseable cursor is 400 |
| documentation → graph-author / operator behaviour | What the guide and manifests teach determines whether secrets land in Gate payloads and whether restarts lose work | Warnings placed with the authoring instructions; 2× rule in all operator docs |
| evidence record → phase seal | A gate recorded as passed that was not run is a false assurance | Unrun Postgres cases named in WINDOWS.md and routed to CI |

---

## Threat Register

Status legend: **closed** = mitigation located in the implementation (evidence column), or
accepted risk recorded in the Accepted Risks Log below.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status | Evidence |
|-----------|----------|-----------|----------|-------------|------------|--------|----------|
| T-24-01 | Spoofing | `WarEngine::resume_with` parley-id matching | high | mitigate | `UnknownParleyId` checked only against the loaded thread's own `AwaitingInput` parleys; no global lookup | closed | `engine/mod.rs:1492`; test `resume_with_parley_ids_are_scoped_to_the_requested_thread` |
| T-24-02 | Information Disclosure | `ParleyRequest.payload`/`prompt` persisted verbatim | medium | mitigate | Engine never writes a credential into a `ParleyRequest`; author warning in mdBook | closed | `docs/src/user-guides/parley-and-chronicle.md:295`; 24-12 manual credential review |
| T-24-03 | Tampering | Resume against a redeployed graph | high | mitigate | `resume_with` runs the same `GraphMismatch` fingerprint check as `resume` before any state change | closed | `engine/mod.rs:1365`; test at `mod.rs:4688`, `mod.rs:6515` |
| T-24-04 | Denial of Service | Suspended thread holding engine resources | medium | mitigate | Every peer handle is joined before the parley branch persists `AwaitingInput` and returns; only the Waypoint is retained | closed | `engine/superstep.rs:2190-2215`; test `e2e2_suspended_thread_holds_no_engine_resources` |
| T-24-05 | Information Disclosure | `GateRequestTemplate.payload_template` rendered into persisted payload | medium | mitigate | Renderer copies only author-templated Battlefield fields; no engine credential reachable; mdBook warning | closed | `parley-and-chronicle.md:295-299` |
| T-24-06 | Elevation of Privilege | `on_expire: ResumeWithDefault` bypassing an approval gate | high | mitigate | Default validated at graph-validate time by the SAME shared `validate_parley_value_for_kind` that `resume_with` uses; unknown kinds fail closed | closed | `engine/graph.rs:997`, `graph.rs:1654-1700`; `mod.rs:1799` |
| T-24-07 | Tampering | Gate routing properties omitted from the graph fingerprint | high | mitigate | `;gates:` section hashed under bumped `v4` tag; redeploy fails closed with `GraphMismatch` | closed | `engine/graph.rs:1507`; v4 pin tests `graph.rs:2208`, `2388`, `2572` |
| T-24-08 | Tampering | Fingerprint field-boundary collision | medium | mitigate | Every field goes through length-prefixed `push_field` | closed | `graph.rs:1425-1483`; tests `fingerprint_distinguishes_length_prefix_collision_*` |
| T-24-09 | Spoofing | Schema field shadowing a `parley.`-namespaced placeholder | high | mitigate | Resolver never falls through to the Battlefield for `parley.`; `WarGraph::validate` rejects the reserved prefix — two independent controls | closed | `engine/input_mapping.rs:182,238-242`; `engine/graph.rs:903-925` |
| T-24-10 | Elevation of Privilege | LLM-authored `ResumeWithDefault` value bypassing approval | high | mitigate | Directive path calls the same shared per-kind validator; invalid value rejected at raise time | closed | `engine/directive_parser.rs:207` |
| T-24-11 | Tampering | Malformed envelope degrading a parley into edge resolution | high | mitigate | Malformed entry routes through `OnParseError`; `NextStep::Parley` never coerced to `Edges`; validation failure is a hard error | closed | test `envelope_parley_malformed_shape_uses_on_parse_error_policy`; 24-03 SUMMARY §Decisions |
| T-24-12 | Information Disclosure | Model-authored `prompt`/`payload` served over HTTP | medium | mitigate | Content never enriched with credentials; mdBook warning | closed | `parley-and-chronicle.md:295`; 24-12 credential review |
| T-24-13 | Tampering | Malformed/oversized StateEdit mutating undeclared fields | high | mitigate | Response deserialized to `StateDelta`, checked field-by-field against the schema; `ResponseShapeInvalid` applies nothing | closed | `engine/mod.rs:1794-1818`, `mod.rs:1504` |
| T-24-14 | Elevation of Privilege | Choice response outside offered options | high | mitigate | Choice accepted only if a member of that request's own `choices`, checked before any write | closed | `engine/graph.rs:1666-1677` |
| T-24-15 | Spoofing | Answering another thread's parley | high | mitigate | Ids matched only against the loaded thread's outstanding parleys | closed | test `resume_with_parley_ids_are_scoped_to_the_requested_thread` (`mod.rs:4724`) |
| T-24-16 | Elevation of Privilege | Expiry default silently approving a gate | high | mitigate | Default validated by the shared validator; substitution recorded with `responded_by: None`, `defaulted: true` | closed | `engine/mod.rs:1455-1457`; `parley.rs:92-93` |
| T-24-17 | Repudiation | Resumed run with no record of who answered | medium | mitigate | `responded_by`, `responded_at`, `defaulted` persisted on `ParleyResponse` | closed | `core/.../parley.rs:208-216` |
| T-24-18 | Spoofing | Cross-thread response delivery under concurrency | high | mitigate | Stress test asserts cross-thread id is `UnknownParleyId` and each thread's Battlefield carries only its own value | closed | `tests/integration/parley_resume_stress_test.rs:231-371` |
| T-24-19 | Denial of Service | Deadlocked concurrent resume hanging the suite | medium | mitigate | Both stress scenarios wrapped in `tokio::time::timeout(30s)` | closed | `parley_resume_stress_test.rs:201,232` |
| T-24-20 | Tampering | Resuming against stale in-process state | high | mitigate | Cross-process assertions read through a fresh store handle via `WaypointPort::latest`, never engine return values | closed | `tests/integration/multi_parley_suspension_test.rs:241-249,431` |
| T-24-21 | Tampering | `ThreadId::child_on_branch` identity collision | high | mitigate | Same length-prefixed injective encoding as `ThreadId::child`, pinned by injectivity tests | closed | `waypoint.rs:223`; tests `child_on_branch_is_injective`, `child_thread_derivation_is_injective_under_adversarial_names` |
| T-24-22 | Tampering | Pre-`fork_of` payload deserializing into a branch | medium | mitigate | `#[serde(default)]` on `fork_of` plus strip-key test | closed | `waypoint.rs:702-703`; test `waypoint_payload_without_fork_of_deserializes_as_none` |
| T-24-23 | Information Disclosure | Branch history leaking via marker-based lookup | medium | mitigate | Isolation comes only from the derived thread id; `checkpoint_ns` is never a lookup key (no SQL column, round-trip only) | closed | `storage/waypoint/postgres.rs:30`; no lookup call sites in storage/application |
| T-24-24 | Denial of Service | Retention pruning a suspended branch's protected Waypoint | medium | mitigate | Retention protection matches any `AwaitingInput { .. }`; branch-resident case tested | closed | `storage/waypoint/retention.rs:215`; test `retention_protects_awaiting_input_on_any_branch` |
| T-24-25 | Tampering | Fork edit mutating the original chain | high | mitigate | `replay`/`fork` only append; mainline byte-identical; second replay does not disturb the first | closed | tests `replay_leaves_the_mainline_byte_identical`, `replay_twice_is_safe` |
| T-24-26 | Tampering | Fork edit naming an undeclared schema field | high | mitigate | Edit merged through schema dispatch; `BattlefieldError::UnknownField` before the first forked superstep, nothing persisted | closed | test `fork_rejects_an_edit_the_schema_does_not_accept` (`mod.rs:6526-6549`) |
| T-24-27 | Information Disclosure | Branch reading the mainline subgraph child's history | high | mitigate | Branch children run under `ThreadId::child_on_branch`; no marker lookup path | closed | `engine/superstep.rs:487-503` |
| T-24-28 | Tampering | `replay`/`fork` against a redeployed graph | high | mitigate | Shared `replay_or_fork` checks the fingerprint first; `GraphMismatch` with nothing persisted | closed | `engine/mod.rs:1683-1705`; test `replay_rejects_fingerprint_mismatch` |
| T-24-29 | Information Disclosure | `ChronicleService` exposing Waypoint content beyond caller rights | medium | mitigate | Service performs no authorisation by design; enforced at the HTTP `route_layer`; split documented | closed | `src/application/services/chronicle.rs:10-11`; `thread_controller.rs:686` |
| T-24-30 | Tampering | Aborted node contributing a partial delta | high | mitigate | Aborted results discarded before merge; record reads `Skipped` | closed | `engine/superstep.rs:1702-1746`; test `over_grace_node_is_aborted_and_recorded_skipped` |
| T-24-31 | Tampering | Duplicate side effects from re-run after abort | high | mitigate | Skipped nodes' edges stay `Pending` so resume re-executes exactly once | closed | test `resume_reruns_the_skipped_node_exactly_once` |
| T-24-32 | Denial of Service | Hung node holding the process past grace | high | mitigate | `cancel_and_wait(grace)` returns at the deadline; `JoinHandle::abort` releases stragglers | closed | `engine/shutdown.rs:162`; `src/bin/paladin-server.rs:476-478` |
| T-24-33 | Tampering | Shutdown config leaking into graph identity | high | mitigate | Fields never enter `EngineLimits` or the fingerprint | closed | `src/config/engine.rs:197`; test `shutdown_grace_does_not_change_the_graph_fingerprint` |
| T-24-34 | Denial of Service | Misconfigured grace stalling restarts | medium | mitigate | `validate()` caps at `MAX_SHUTDOWN_GRACE_SECS`; `Duration::ZERO` explicitly supported | closed | `src/config/engine.rs:145`; tests `engine_config_validates_shutdown_grace`, `shutdown.rs:444` |
| T-24-35 | Denial of Service | Pod SIGKILLed mid-grace losing the Halted Waypoint | high | mitigate | Both manifests set `terminationGracePeriodSeconds: 60`; 2× rule documented | closed | `k8s/deployment.yaml:34`, `k8s/server/deployment.yaml:34`, `k8s/README.md:172-175` |
| T-24-36 | Denial of Service | Hung run holding process open on shutdown | medium | mitigate | Wait bounded by `cancel_and_wait(grace)`; `graceful_shutdown=false` removes the wait | closed | `paladin-server.rs:93-95,195-198,476` |
| T-24-37 | Repudiation | Operator unable to tell whether a restart lost work | medium | mitigate | Production guide describes SIGTERM behaviour and the M-B-02 switch; over-grace nodes recorded `Skipped` and re-listed | closed | `docs/src/deployment/production.md:312,350,385` |
| T-24-38 | Information Disclosure | Deployment docs/manifests carrying a credential | medium | mitigate | Only env-var names and numeric defaults added; `secret.yaml.example` untouched in the phase diff | closed | phase diff over `k8s/` touches README + two deployment.yaml only |
| T-24-39 | Tampering | Continuation running against the wrong graph | high | mitigate | Graph resolved strictly by the thread's stored fingerprint; `GraphNotRegistered` with no fallback | closed | `src/application/services/parley/adapter.rs:111,598-611` |
| T-24-40 | Denial of Service | Unbounded background continuation surviving shutdown | high | mitigate | Spawned task registers with `ShutdownCoordinator` and holds a `RunGuard` | closed | `adapter.rs:146-150`; test `spawned_continuation_is_registered_with_the_coordinator` |
| T-24-41 | Information Disclosure | Postgres connection string in serialized config or `Debug` | high | mitigate | Postgres variant carries `url_env` (the env-var NAME); `validate()` resolves at startup without storing | closed | `src/config/waypoint_store.rs:25-45,116-125` |
| T-24-42 | Elevation of Privilege | Validation failure accepted then failing silently in background | high | mitigate | Validation synchronous and total before spawn; error returns with nothing persisted | closed | `adapter.rs:127`; test `adapter_validates_synchronously_and_returns_typed_errors` |
| T-24-43 | Repudiation | Distinct validation failures collapsing into one error | medium | mitigate | Every `EngineError` validation variant maps to a distinct `ParleyError` | closed | tests `every_engine_error_maps_to_a_distinct_parley_error`, `parley_error_covers_every_validation_case` |
| T-24-44 | Spoofing | Unauthenticated access to thread state or resume | high | mitigate | `thread_openapi_router` applies the same `require_authentication` `route_layer` as agents | closed | `thread_controller.rs:681-688`; test `thread_routes_require_authentication` |
| T-24-45 | Information Disclosure | `state` returning raw prompts or an author-embedded secret | high | mitigate | Endpoint adds no credential of its own; author warning and interim posture documented | closed | `parley-and-chronicle.md:295-308` |
| T-24-46 | Elevation of Privilege | Any authenticated caller answering any approval gate | medium | accept | Accepted per D-24 as interim posture; **shipped posture is narrower**: resume requires `require_admin` (403 otherwise) pending PLAT-06 | closed (accepted, narrowed) | `thread_controller.rs:554`; tests `post_resume_with_non_admin_role_is_403`, `post_resume_with_admin_role_is_202` |
| T-24-47 | Denial of Service | Unbounded `limit` on history | medium | mitigate | `MAX_HISTORY_LIMIT = 100`; larger is 400 | closed | `thread_controller.rs:151,633-634` |
| T-24-48 | Denial of Service | Resume request holding a connection for a whole run | medium | mitigate | Handler returns 202 immediately; continuation runs in the shutdown-bounded background task | closed | `thread_controller.rs:588`; test `post_resume_returns_202_with_thread_and_state_url` |
| T-24-49 | Information Disclosure | Connection string or key in a 501 envelope | high | mitigate | 501 hints name the config KEY only | closed | `thread_controller.rs:137-148`; test `thread_routes_return_501_when_no_backend_is_wired` |
| T-24-50 | Information Disclosure | Authors templating credentials into a Gate payload | high | mitigate | Explicit warning placed with the Gate authoring instructions | closed | `parley-and-chronicle.md:295-299` |
| T-24-51 | Repudiation | Postgres contract case recorded as passed without Docker | high | mitigate | 24-12 SUMMARY records the cases as routed to CI `postgres-integration`, not verified locally; WINDOWS.md row filed | closed | `24-12-SUMMARY.md:43,176,208` |
| T-24-52 | Information Disclosure | Credential reaching a log, error envelope or `Debug` output | high | mitigate | Manual credential-handling review performed and recorded, no findings | closed | `24-12-SUMMARY.md:143` |
| T-24-53 | Spoofing | A security scan reading as coverage it does not provide | medium | mitigate | No Snyk step in any workflow; record states `make security` scans dependencies and clippy is a lint | closed | `.github/workflows/` (no Snyk step; codeql.yml comment only); `24-12-SUMMARY.md:139,143` |
| T-24-13-01 | Information Disclosure | Phase-closing docs misstating the auth posture | medium | mitigate | CHANGELOG, MIGRATION §9.6 and the mdBook page all state admin-gated resume (403) and PLAT-06 successor | closed | `MIGRATION.md:202-211`; `CHANGELOG.md:123-126`; `parley-and-chronicle.md:277-308` |
| T-24-13-02 | Elevation of Privilege | `POST /v1/threads/{id}/resume` control relaxed alongside a doc edit | high | mitigate | `require_admin` re-asserted on `resume_thread`; the three role tests re-run | closed | `thread_controller.rs:554`; tests `post_resume_with_non_admin_role_is_403`, `post_resume_with_admin_role_is_202`, `get_thread_state_with_non_admin_role_is_200_not_403` |
| T-24-13-03 | Information Disclosure | Gate `payload_template` returned verbatim to any authenticated caller | medium | accept | Unchanged accepted interim posture (D-24); warning blockquote retained; PLAT-06 successor | closed (accepted) | `parley-and-chronicle.md:295-308` |
| T-24-13-04 | Tampering | `openapi.rs` drift-guard test edit outside the test module | low | mitigate | 24-13 commit `e462fcb0` changes only the test doc comment and status array inside `mod tests` | closed | `git show e462fcb0 -- crates/paladin-web/src/openapi.rs` |
| T-24-13-05 | Repudiation | Documentary record vs shipped code | medium | mitigate | `openapi_thread_paths_document_every_status` asserts `403`, so a regeneration that drops it fails CI | closed | `crates/paladin-web/src/openapi.rs:148-170` |
| T-24-14-01 | Tampering | `MusterProgress` merge ordering in the superstep join loop (CR-02 fix) | high | mitigate | Human diff read of commit `9802ce60` recorded with verdict; regression test re-run green | closed | `24-14-SUMMARY.md` §Human Verdict; test `shutdown_grace_abort_mid_muster_preserves_progress_for_resume` |
| T-24-14-02 | Repudiation | The phase's evidence record | medium | mitigate | Verdict, shapes and verbatim command output recorded | closed | `24-14-SUMMARY.md:52-108` |
| T-24-14-03 | Denial of Service | Resumed run after a mid-Muster abort | medium | mitigate | Exactly-once merge covered in both failure directions; non-Muster contract pinned | closed | `24-14-SUMMARY.md:82-108`; test `shutdown_grace_abort_mid_muster_preserves_progress_for_resume` |
| T-24-SC | Tampering | package-manager installs (plans 24-01 … 24-12) | low | accept | No new third-party package entered the build; see Accepted Risks Log for the one dev-dependency line | closed (accepted) | `git diff 19e51fe2~1 HEAD -- Cargo.toml Cargo.lock` |
| T-24-13-SC | Tampering | package-manager installs (plan 24-13) | low | accept | No install task; `Cargo.toml` untouched | closed (accepted) | phase diff |
| T-24-14-SC | Tampering | package-manager installs (plan 24-14) | low | accept | No install task; `Cargo.toml` untouched | closed (accepted) | phase diff |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Totals:** 64 register entries — 35 high, 25 medium, 4 low. 59 mitigated and verified, 5 accepted. **threats_open: 0.**

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-24-01 | T-24-46 | D-24 scopes this phase to "authenticated callers, any role"; per-thread ownership scoping is PLAT-06 (Phase 27). The shipped code narrowed the accepted posture during review: `POST /v1/threads/{id}/resume` requires `require_admin` and answers 403 to a non-admin caller, while reads stay any-authenticated-role. Documented in the mdBook page, MIGRATION §9.6 and CHANGELOG. | Plan 24-11 (D-24); narrowed in 24-REVIEW-FIX | 2026-09-05 |
| AR-24-02 | T-24-13-03 | Gate `payload_template` is returned verbatim by `GET /v1/threads/{id}/state` to any authenticated caller. Same interim posture as AR-24-01; the "never template a secret into a Gate payload" warning is kept with the authoring instructions; PLAT-06 is the named successor. | Plan 24-13 | 2026-09-05 |
| AR-24-03 | T-24-SC | Supply chain. No new `[[package]]` entered `Cargo.lock`. One new dependency line was added: `tower = "0.5"` under the root crate's `[dev-dependencies]` (plan 24-11) so `paladin-server.rs`'s own test module can drive routers via `tower::util::oneshot`; the crate was already resolved at `0.5.3` through `paladin-web`. This contradicts plan 24-10's literal wording ("no new dependency line in the root crate") but not the package-legitimacy concern the entry guards. `make security` (cargo-audit + cargo-deny) passed in 24-12. | Plans 24-01 … 24-12 | 2026-09-05 |
| AR-24-04 | T-24-13-SC | No install task, no dependency change in plan 24-13. | Plan 24-13 | 2026-09-05 |
| AR-24-05 | T-24-14-SC | No install task, no dependency change in plan 24-14. | Plan 24-14 | 2026-09-05 |

*Accepted risks do not resurface in future audit runs.*

---

## Verification Notes

- **Method.** Grep-depth (L1) verification per the secure-phase short-circuit rule: ASVS level 1,
  register authored at plan time, zero open entries after preliminary classification. The
  `gsd-security-auditor` subagent was therefore not spawned. Each row's Evidence column names the
  file and line, test, or document line that pins the mitigation.
- **Targeted test re-run.** All security-pinning tests named in the register were re-run live on 2026-09-05 with a warm build cache; every run was green. Counts per invocation: `paladin-ai-core` 3 passed (`child_on_branch_is_injective`, `child_thread_derivation_is_injective_under_adversarial_names`, `waypoint_payload_without_fork_of_deserializes_as_none`); `paladin-ports` 1 passed; `paladin-storage` 1 passed; `paladin-battalion` 12 passed (resume scoping, fingerprint mismatch, fork schema rejection, mainline byte-identity, replay idempotence, over-grace abort, exactly-once re-run, length-prefix collision, malformed-envelope policy, mid-Muster abort, RunGuard drop); `paladin-web --all-features` 6 passed (authentication layer, admin 403/202, non-admin read 200, 501 hints, OpenAPI 403 drift guard); root `--features web-server` 9 passed (error mapping, synchronous validation, coordinator registration, fingerprint indifference, grace bounds, waypoint-store env-var name); integration binaries `parley_resume_stress`, `multi_parley_suspension`, `e2e_approval_gate` 90 passed. Total 122 passed, 0 failed. The Postgres Tier-2 cases were not exercised locally (no Docker daemon), consistent with 24-12's WINDOWS.md row.
- **Noteworthy observations.**
  - T-24-46 shipped stronger than planned: resume is admin-gated (403) rather than any-role. The
    register records the plan's accept disposition and the narrowed shipped posture side by side.
  - The Postgres Tier-2 contract cases added in 24-06 self-skip locally (no Docker daemon) and are
    routed to CI's `postgres-integration` job via WINDOWS.md; the retention branch-protection
    test (T-24-24) runs on the SQLite/in-memory backends locally.
  - Known gap carried from `security.instructions.md`: there is still no merge-gating Rust SAST.
    The manual credential-handling review in 24-12 is the primary control for T-24-41/49/52 and
    was performed with no findings.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-05 | 64 | 64 | 0 | /gsd-secure-phase 24 (Claude, L1 grep-depth + targeted test re-run) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-05
