---
phase: 53
slug: tenant-isolation-closure
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-10-31
verified_head: 5f3cb4222
---

# Phase 53 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All eight PLAN files (53-01 through 53-08) carry a
`<threat_model>` block (42 threats, T-53-01 through T-53-42); all eight SUMMARY files report
`## Threat Flags: None` (no new network endpoint, auth path, file access or schema change beyond
the planned register -- the phase narrows existing routes and adds boot-time guards). The
verification below is L1 grep-depth against the tree at `5f3cb4222`, which the ASVS level 1
configuration accepts as sufficient when no threat at or above the `high` block threshold
remains open. Each "mitigate" row names the function or test that was confirmed present; each
"accept" row names where the acceptance is recorded.

The one judgment-tier control the plans assign to this workflow (T-53-31 / AC-N5, "no escape
hatch and no no-op engine factory") was performed by reading the shipped wiring, not by grep:
see the T-53-31 row and the note under Review-Surfaced Residuals.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HTTP client -> run / thread routers | A user-role principal of one tenant names another tenant's `thread_id` on `POST /v1/runs`, the fork path, or any `/v1/threads/{id}` route | Thread ids (caller-chosen or UUIDv7); run bodies |
| library caller -> `RunSubmissionService` | An embedder calls `submit`/`fork`/`cancel` with a `PrincipalRef` and no HTTP controller in the path | `PrincipalRef`, run/thread ids |
| run store -> run / thread controller | Repository errors can carry backend text (connection detail, row shape, another tenant's attribution) | `RunRepositoryError` text; run rows |
| waypoint store -> thread controller | Waypoint rows and store errors of any schema version | Waypoint rows; `WaypointStoreError` text |
| run / thread controller -> HTTP client | Status codes and error bodies are visible to any authenticated principal | Fixed error messages; 404 vs 500 vs 403 shape |
| HTTP client -> `GET /v1/threads` | Any authenticated principal pages the deployment's thread list | Thread summaries; keyset cursor (`last_updated_at`) |
| run store -> list walk | One owner lookup per examined thread, bounded per request | Latest-run attribution per thread |
| HTTP client -> `GET /agents/{id}/jobs/{job_id}` | Any authenticated principal can present any job id under any agent id | Job results |
| embedder -> `paladin_web::agent_controller::get_job` | A downstream router may still mount the unscoped handler | Job results (unscoped path, documented) |
| operator config -> boot | YAML/env values become principals, tenants and the Treasurer wiring | API key names/values, tenant ids, allowance policy |
| schedule store -> boot log | Stored schedule rows are read at boot and summarized into operator log lines | Schedule ids (never row text) |
| evidence -> governance ledgers | WINDOWS rows and dispositions are read by `/gsd-ship` and Phase 57 as proof | Test names, result lines, head SHA |
| local tool run / CI job -> decision record | Phase 58's tag audit trusts the recorded semver lines | `cargo semver-checks` output lines, head SHA |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-53-01 | Tampering | `ensure_thread_visible` (submit/fork onto a foreign thread) | high | mitigate | `ensure_thread_visible` (`src/application/services/run/submission.rs`) calls `RunReadScope::permits_thread(Some(&latest))` and returns `UnknownThread`; tests `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404` and `cross_tenant_fork_is_the_unknown_thread_404` present in `http_surface_tests.rs` | closed |
| T-53-02 | Information disclosure | refusal shape on submit/fork | medium | mitigate | Refusal is the `UnknownThread` 404 (never `ThreadBusy`/403); asserted on code and message by the two tests above | closed |
| T-53-03 | Elevation of privilege | `RunSubmissionService::cancel` without a router | high | mitigate | Service-layer scope check in `cancel` before `authorize_invocation` and `request_cancel`; `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` present and re-run (53-01-SUMMARY, ADR-0063 Evidence) | closed |
| T-53-04 | Information disclosure | unused versus hidden thread id on submit (thread birth) | low | accept | Recorded in `ensure_thread_visible` rustdoc (40-CONTEXT D-14, UUIDv7 default), `53-TENANT-EVIDENCE.md` `## Residuals`, and ADR-0063 "Accepted residuals"; see Accepted Risks AR-53-01 | closed |
| T-53-05 | Information disclosure | corrupt latest-run row on submit | low | accept | Body is the fixed `"run store error"` (no backend text); D-09 confines the 404 mapping to the two load helpers; recorded in evidence `## Residuals` and ADR-0063; see AR-53-02 | closed |
| T-53-06 | Spoofing | tenant asserted by the request | high | mitigate | Scope derives only from the authenticated `PrincipalRef`; `permits_thread` takes no tenant argument (ADR-0054 rule 1); the cross-tenant tests send no tenant field | closed |
| T-53-07 | Information disclosure | `load_visible_run` on an unreadable row | high | mitigate | `run_repo_error_for_scope` (`crates/paladin-web/src/run_controller.rs`): `Serialization` under a `Tenant` scope is the missing-run 404; `serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin` present (OpenAPI-derived route set) | closed |
| T-53-08 | Information disclosure | backend text in error bodies | high | mitigate | Every branch of `redact_store_error_for_scope` returns a fixed message or the 404; detail goes to `log::error!` only; AC-10 negative assertions in the route tests | closed |
| T-53-09 | Information disclosure | outage-class errors as an existence oracle | medium | mitigate | Only classifier-declared hideable errors become 404; `run_backend_error_stays_a_fixed_500_for_a_tenant` and `run_list_serialization_stays_a_fixed_500_for_every_role` (ADR-0063 Evidence) | closed |
| T-53-10 | Information disclosure | `UnknownSchemaVersion` row after a downgrade | low | accept | `run_repo_error_for_scope` rustdoc (l.738) records "deliberately not hideable (D-09, PD-02-2)"; ADR-0063 "Accepted residuals"; see AR-53-03 | closed |
| T-53-11 | Repudiation | operator visibility of hidden corrupt rows | low | mitigate | The hidden branch of `redact_store_error_for_scope` logs at `error` with route context before answering the 404 | closed |
| T-53-12 | Information disclosure | `GET /v1/threads/{id}`, `/state`, `/history` (IDOR) | high | mitigate | `load_visible_thread` (`thread_controller.rs`) on every `{id}` handler; `every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404` compares against the router's own OpenAPI operation set and asserts status/headers/body equality with a missing thread | closed |
| T-53-13 | Tampering | `POST .../resume`, `POST .../fork`, `DELETE /v1/threads/{id}` on a foreign thread | high | mitigate | Gate precedes `require_admin` on resume/delete and precedes the fork submission; service re-check on fork (`ensure_thread_visible`); matrix rows cover all three (53-VERIFICATION SC1) | closed |
| T-53-14 | Information disclosure | 403 versus 404 existence oracle on admin routes | medium | mitigate | D-06 order (visibility gate first); `post_resume_by_a_foreign_user_is_404_not_403` present; matrix asserts no 403 to the foreign key | closed |
| T-53-15 | Information disclosure | backend text in thread error bodies (seven sites) | high | mitigate | `thread_store_error` and `waypoint_error_for_scope` present in `thread_controller.rs`; `run_repo_error_for_scope` shared with the run routes; AC-10 negative assertions | closed |
| T-53-16 | Information disclosure | corrupt Waypoint or run row as an existence oracle | medium | mitigate | Hideable classifiers return the 404 for a `Tenant` scope; `serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin` present | closed |
| T-53-17 | Spoofing | tenant hint in header/query/body on thread routes | high | mitigate | Scope only from `Principal::read_scope()`; `tenant_hints_in_headers_or_query_change_nothing_on_thread_routes` present | closed |
| T-53-18 | Denial of service | no run store wired | low | accept | D-07 fail closed (threads hidden from non-admin; `thread_routes_fail_closed_for_non_admin_without_a_run_store`, `list_threads_fails_closed_for_non_admin_without_a_run_store`); operator action documented in MIGRATION.md §11.1; see AR-53-04 | closed |
| T-53-19 | Information disclosure | race between check and read (run submitted mid-request) | low | accept | SPEC edge R1/concurrency dismissed: at most one request stale; mutating paths (`fork`, `submit`, `cancel`) re-check inside the service; see AR-53-05 | closed |
| T-53-20 | Information disclosure | thread enumeration through the list | high | mitigate | `visible_thread_page` keeps only threads `permits_thread` admits for a non-admin scope; `list_threads_omits_foreign_and_zero_run_threads` and `list_threads_returns_the_empty_page_shape_when_nothing_is_visible` (ADR-0063 Evidence) | closed |
| T-53-21 | Denial of service | unbounded walk (N+1 over a large store) | medium | mitigate | `MAX_EXAMINED_THREADS` const in `thread_controller.rs` bounds examined threads per request; `list_threads_examination_cap_returns_a_short_page_with_a_cursor` | closed |
| T-53-22 | Denial of service | one tenant's corrupt row failing everyone's list | medium | mitigate | `Serialization` on an owner lookup is skipped and logged for a `Tenant` scope; `list_threads_skips_a_corrupt_thread_for_a_tenant`. Note: review finding WR-01 observes that `UnknownSchemaVersion` (a downgrade condition, not corruption) is not skipped and aborts the walk; see Review-Surfaced Residuals | closed |
| T-53-23 | Information disclosure | cursor as a side channel (timestamps of hidden threads) | low | accept | Cursor is the last examined thread's `last_updated_at` (reveals that some thread was updated at that instant, not its id, tenant or content); accepted with D-04's keyset walk in 53-04-SUMMARY Threat Flags; see AR-53-06 | closed |
| T-53-24 | Spoofing | tenant hint in header/query on the list | high | mitigate | Scope only from `Principal::read_scope()`; `list_threads_ignores_tenant_hints` present | closed |
| T-53-25 | Information disclosure | job result read by a foreign tenant | high | mitigate | `get_job_scoped` is the only routed handler (`agent_openapi_router` `routes!(get_job_scoped)`); reads `JobStore::get_visible(job_id, agent_id, scope)`; owner recorded by `create_owned`; `job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass` present | closed |
| T-53-26 | Information disclosure | existence oracle between hidden and unknown job | medium | mitigate | Same `unknown job` 404 for hidden, agent-mismatched, evicted and unknown, compared byte for byte in the matrix test | closed |
| T-53-27 | Elevation of privilege | embedder still routing the unscoped `get_job` | medium | accept | Signature kept for semver (PD-05-1); `get_job` rustdoc says "Superseded by `get_job_scoped`" and that `#[deprecated]` waits for the next minor (MIGRATION.md §11.7); the shipped server routes only the scoped handler; see AR-53-07 | closed |
| T-53-28 | Spoofing | tenant hint in header/query on job poll | high | mitigate | Scope only from `Principal::read_scope()`; `job_poll_ignores_tenant_hints` present | closed |
| T-53-29 | Tampering | owner map drifting from the job map on eviction | low | mitigate | One insert-and-evict body removes both maps; `evicted_owned_job_is_gone_for_every_scope` present in `job_store.rs` | closed |
| T-53-30 | Elevation of privilege / Repudiation | Treasurer silently not enforced (audit G2) | high | mitigate | `assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory())?` at `run_api_wiring.rs:1035`, after `with_treasurer` and before `pool.spawn`; `paladin-server` builds the API before `TcpListener::bind`; tests `treasurer_assertion_refuses_a_configured_treasurer_without_an_engine_factory`, `pool_built_without_an_engine_factory_is_refused_with_a_treasurer`, `shipped_wiring_with_a_treasurer_boots` | closed |
| T-53-31 | Elevation of privilege | escape hatch (config key or no-op factory) | high | mitigate | Judgment check performed by this workflow on `5f3cb4222`: no config key touches the assertion (grep of `src/config/` for treasurer enforce/skip/disable/bypass finds none); `RunWorkerPool::has_engine_factory` reads the real `Option` field; the shipped `engine_factory` (`run_api_wiring.rs:910-923`) builds a real `WarEngine::new(paladin_port, store)` with durability, grace and the per-run cancellation token, and is applied unconditionally at l.1002; the refusal text is the constant `TREASURER_UNENFORCED_MESSAGE`. No escape hatch found | closed |
| T-53-32 | Repudiation | factory present but `with_treasurer` removed | low | accept | Outside the locked D-00f contract; shipped wiring always calls `with_treasurer` when a Treasurer exists (l.1030); `build_run_api_wires_a_treasurer_only_when_allowances_are_configured` present; recorded in ADR-0063 "Accepted residuals"; see AR-53-08 | closed |
| T-53-33 | Spoofing / Elevation | principal or tenant equal to a sentinel (`open-access`, `unattributed`, `anonymous`) | medium | mitigate | Exact, case-sensitive rejection in `AuthConfig::validate` (`src/config/agents.rs:205-265`) at every position; `validate_rejects_reserved_literals_in_every_position` and `validate_accepts_case_variants_of_reserved_literals` present | closed |
| T-53-34 | Information disclosure | key value or row text in boot/validation messages | high | mitigate | Messages name the entry (`api_keys[#index]`, `api_keys[name]`, `bearer_token`) and the offending literal, never a key value (`agents.rs:209,226,261`); scan error line is fixed text; manual credential review recorded in `53-TENANT-EVIDENCE.md` (l.154) per `security.instructions.md` | closed |
| T-53-35 | Tampering | the advisory scan writing to the store | medium | mitigate | `scan_unattributed_schedules_paged` calls only `list` (recording-wrapper test); SQLite before/after dump equality (53-06-SUMMARY) | closed |
| T-53-36 | Denial of service | scan blocking boot on a corrupt row | low | mitigate | `scan_unattributed_schedules_paged` returns `outcome` on `Err(_)` (page error logged once, scan stops); never returns an error to boot. Note: review IN-03 observes no progress guard against a non-advancing cursor (adapter misbehaviour, not a row condition); see Review-Surfaced Residuals | closed |
| T-53-37 | Repudiation | WINDOWS rows closed without proof | medium | mitigate | `53-TENANT-EVIDENCE.md` `## Closing-test re-run (plan 53-07, T-53-37)` records result lines on the head before `windows fixed`; rows cite test names | closed |
| T-53-38 | Spoofing (assurance) | CodeQL or Snyk read as coverage | medium | mitigate | Evidence states CodeQL is advisory-only and that no static taint analysis gates a merge (l.154); zero occurrences of "snyk" in the evidence file (negative grep run by this workflow) | closed |
| T-53-39 | Information disclosure | docs promising less or more isolation than shipped | low | mitigate | `docs/src/user-guides/parley-and-chronicle.md` "Who can see a thread" block matches the latest-run rule; no "not yet tenant-scoped" thread posture note remains in `docs/src` (the one remaining "not tenant-scoped" line is about `GET /v1/schedules`, which is accurate) | closed |
| T-53-40 | Repudiation | a "green" decision without evidence | high | mitigate | `53-VERSION-DECISION.md` records the twelve verbatim `cargo semver-checks` lines with N > 0 and the head SHA (`4ef9ddd2a`); zero-check runs fail by the script's own tripwire (Phase 52 T-52-01) | closed |
| T-53-41 | Tampering | a break slipping through as "additive" | high | mitigate | Patch-level gate on all twelve packages; allowlist and MIGRATION §11.2 checked empty (53-VERSION-DECISION.md); RED stops the plan | closed |
| T-53-42 | Spoofing (assurance) | lines copied from a different head | medium | mitigate | Head SHA recorded and measured on `4ef9ddd2a` itself with no earlier lines reused (53-08-SUMMARY) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Totals:** 42 threats; 34 mitigate (all closed); 8 accept (all documented, closed); 0 open.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-53-01 | T-53-04 | Thread birth is open: `POST /v1/runs {thread_id}` on an id with no runs succeeds while a foreign thread is 404, so an unused id is distinguishable from a hidden one on that route only. Ids are UUIDv7 by default; tenant-namespaced ids are the deferred promote (40-CONTEXT D-14). Recorded in rustdoc, evidence `## Residuals`, ADR-0063. | plan 53-01 (D-14) | 2026-10-31 |
| AR-53-02 | T-53-05 | A corrupt latest-run row on submit reaches the fixed `500 "run store error"` (no backend text); D-09 confines the role-dependent 404 to the two load helpers. Recorded in evidence `## Residuals`, ADR-0063. | plan 53-01 (D-09) | 2026-10-31 |
| AR-53-03 | T-53-10 | A run row of a newer schema version after a downgrade answers a tenant 500 rather than 404, because a version mismatch is an operator condition, not row corruption (D-09, PD-02-2). Recorded in `run_repo_error_for_scope` rustdoc and ADR-0063. Review WR-01 asks whether the list-walk consequence should be revisited; that is the open human decision in 53-UAT.md. | plan 53-02 (D-09) | 2026-10-31 |
| AR-53-04 | T-53-18 | With no run store wired, thread routes fail closed for non-admin principals (D-07). Operator action documented in MIGRATION.md §11.1; tested. | plan 53-03 (D-07) | 2026-10-31 |
| AR-53-05 | T-53-19 | A run submitted between the visibility check and the read leaves at most one request stale; every mutating path re-checks inside the service (SPEC edge R1). | plan 53-03 | 2026-10-31 |
| AR-53-06 | T-53-23 | The keyset cursor encodes the last examined thread's `last_updated_at`, revealing that some thread was updated at that instant but not its id, tenant or content; inherent to D-04's walk. | plan 53-04 (D-04) | 2026-10-31 |
| AR-53-07 | T-53-27 | The unscoped `get_job` handler keeps its signature for semver (PD-05-1); its rustdoc and MIGRATION §11.7 tell embedders to route `get_job_scoped`; the shipped server routes only the scoped handler; `#[deprecated]` is deferred to the next minor. | plan 53-05 (PD-05-1) | 2026-10-31 |
| AR-53-08 | T-53-32 | A factory present while the wiring drops `with_treasurer` is outside the locked D-00f contract; the shipped wiring always calls it and the wiring test covers it. Recorded in ADR-0063. | plan 53-06 (D-00f) | 2026-10-31 |

*Accepted risks do not resurface in future audit runs.*

---

## Review-Surfaced Residuals (not in the plan register)

`53-REVIEW.md` (gsd-code-reviewer, standard depth, 0 critical / 5 warning / 5 info) and
`53-VERIFICATION.md` already route these to a human decision, persisted as `53-UAT.md`. None
reopens a register threat, none is at the `critical` level, and the reviewer classed each as a
documented residual or edge case rather than a regression. They are listed here so the
security record is complete; the gate above does not count them.

| Ref | Relates to | Summary | Where it stands |
|-----|------------|---------|-----------------|
| WR-01 | T-53-10, T-53-22 | `UnknownSchemaVersion` is not hideable: a tenant gets 500 (existence signal) on single-object routes, and in the list walk one such row in any tenant's thread aborts every non-admin `GET /v1/threads` until repaired. The waypoint side already hides `SchemaVersionUnsupported`. | Human decision pending (53-UAT.md): widen hideable to include `UnknownSchemaVersion`, or skip-and-log it in the walk, or keep and add a MIGRATION note. |
| WR-02 | T-53-05, T-53-12 | "Latest run" is read via `list(limit: 1)`, which decodes `limit + 1` rows, so a corrupt second-newest run row makes the owner's own thread 404 on every gated route and 500 on submit/fork. Availability, not disclosure. | Human decision pending: add a `LIMIT 1` port method, or document next to `latest_run_for_thread`. |
| WR-03 | T-53-12 (design) | Ownership is the latest run's `submitted_by`: an Admin of another tenant forking onto tenant A's thread, or an unattributed scheduled run, transfers visibility away from A. `mixed_tenant_thread_belongs_to_the_latest_runs_tenant` locks this as the chosen rule; the transfer-by-admin path is not called out in the docs. | Human decision pending: document in ADR-0063 and the parley doc, or derive ownership from the first attributed run. |
| WR-04 | T-53-04 | Submit on a caller-chosen thread id is an existence oracle with a side effect (a probe of a missing id creates a thread and consumes allowance). `ThreadId` accepts guessable strings, so the UUIDv7 argument in AR-53-01 holds only for generated ids. | Already an accepted residual (AR-53-01); the reviewer asks for the guessable-id caveat to be added to the ADR and the Thread birth doc. |
| WR-05 | T-53-12, T-53-14 | A hidden thread performs one more store call than a missing thread before the identical 404, so a timing difference exists although status, headers and body match. | Human decision pending: record timing as an accepted residual in ADR-0063, or make both branches perform the same I/O. |
| IN-02 | T-53-30, T-53-31 | The boot assertion cannot fire on the shipped path because `with_engine_factory` is unconditional; it is a tripwire for future edits, and it is blind to a no-op factory. This workflow's judgment read (T-53-31 row) found the shipped factory real. | No action required (reviewer's own verdict). |
| IN-03 | T-53-36 | The schedule scan has no progress guard against a non-advancing cursor from a misbehaving adapter, and the warning line is unbounded in length. | Advisory; optional hardening. |
| IN-05 | T-53-11 | The hidden branch logs the repository error at `error` on every request, so a tenant who knows a corrupt id can generate unbounded log volume; no secret was found in those messages. | Advisory; rate-limit or downgrade to `warn` optional. |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-31 | 42 | 42 | 0 | /gsd-secure-phase 53 (orchestrator, L1 grep-depth; auditor short-circuited per ASVS L1 with a plan-authored register and threats_open: 0) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-31
