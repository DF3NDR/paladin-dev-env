---
phase: 55-correctness-fixes-test-hygiene-lint-bar
reviewed: 2026-11-01
depth: deep
files_reviewed: 154
files_reviewed_list:
  - src/infrastructure/web/run_api_wiring.rs
  - docs/src/architecture/treasurer.md
  - docs/src/getting-started/configuration.md
  - crates/paladin-core/src/platform/container/allowance.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/openapi.json
  - docs/src/api-reference/platform-api.md
  - crates/paladin-storage/src/sqlite_content_repository.rs
  - crates/paladin-storage/src/mysql_content_repository.rs
  - crates/paladin-content/src/adapters/input/file_content_fetcher.rs
  - crates/paladin-content/src/services/mod.rs
  - crates/paladin-content/src/services/content_filtering_service.rs
  - docs/src/user-guides/content-processing.md
  - crates/paladin-core/src/base/service/collection_versioning_service.rs
  - crates/paladin-core/src/base/service/field_version_service.rs
  - crates/paladin-core/src/base/component/element.rs
  - crates/paladin-core/src/base/entity/field.rs
  - crates/paladin-core/src/base/entity/collection.rs
  - crates/paladin-core/src/base/mod.rs
  - src/application/services/paladin/planning_service.rs
  - crates/paladin-core/src/platform/container/planning.rs
  - crates/paladin-llm/src/provider_factory.rs
  - crates/paladin-llm/src/config/layers.rs
  - crates/paladin-llm/src/registry/mod.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/reasoning.rs
  - crates/paladin-llm/src/cadence.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/registry/seeded.rs
  - src/application/services/run/cancel.rs
  - src/application/services/run/cancel_tests.rs
  - crates/paladin-battalion/src/commander.rs
  - scripts/check-no-stub-panics.sh
  - tests/scripts/check-no-stub-panics_test.sh
  - Makefile
  - .github/workflows/ci.yml
  - CHANGELOG.md
  - MIGRATION.md
  - crates/paladin-content/CHANGELOG.md
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-web/CHANGELOG.md
  - docs/src/appendix/doc-coverage-report.md
  - docs/src/user-guides/eval-harness.md
  - docs/schemas/eval-scenario.schema.json
  - .project/v0.11.1/01-carried-gaps-inventory.md
  - crates/paladin-ports/src/input/allowance_admission_port.rs
  - crates/paladin-ports/src/input/assistant_admin_port.rs
  - crates/paladin-ports/src/input/parley_port.rs
  - crates/paladin-ports/src/input/run_event_stream_port.rs
  - crates/paladin-ports/src/input/run_inspector_port.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/input/schedule_admin_port.rs
  - crates/paladin-ports/src/lib.rs
  - crates/paladin-ports/src/output/arsenal_port.rs
  - crates/paladin-ports/src/output/assistant_repository_port.rs
  - crates/paladin-ports/src/output/battalion_port.rs
  - crates/paladin-ports/src/output/cadence_port.rs
  - crates/paladin-ports/src/output/cancellation_probe.rs
  - crates/paladin-ports/src/output/citadel_port.rs
  - crates/paladin-ports/src/output/embedding_port.rs
  - crates/paladin-ports/src/output/file_storage_port.rs
  - crates/paladin-ports/src/output/garrison_port.rs
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/model_profile.rs
  - crates/paladin-ports/src/output/node_cache_port.rs
  - crates/paladin-ports/src/output/notification_port.rs
  - crates/paladin-ports/src/output/orchestrator_port.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/paladin-ports/src/output/queue_port.rs
  - crates/paladin-ports/src/output/rate_limit_hints.rs
  - crates/paladin-ports/src/output/run_queue_port.rs
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-ports/src/output/run_schedule_repository_port.rs
  - crates/paladin-ports/src/output/run_trace_port.rs
  - crates/paladin-ports/src/output/sanctum_port.rs
  - crates/paladin-ports/src/output/spend_guard.rs
  - crates/paladin-ports/src/output/spend_scope.rs
  - crates/paladin-ports/src/output/streaming_executor_port.rs
  - crates/paladin-ports/src/output/structured_executor_port.rs
  - crates/paladin-ports/src/output/token_counter_port.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-ports/src/output/treasury_ledger_port.rs
  - crates/paladin-ports/src/output/treasury_notice_port.rs
  - crates/paladin-ports/src/output/vault_confined.rs
  - crates/paladin-ports/src/output/vault_port.rs
  - crates/paladin-ports/src/output/waypoint_port.rs
  - crates/paladin-ports/src/output/webhook_delivery_port.rs
  - crates/paladin-ports/src/output/workflow_repository_port.rs
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/src/scenario.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/config/bridge.rs
  - crates/paladin-llm/src/config/vision.rs
  - crates/paladin-llm/src/error.rs
  - crates/paladin-llm/src/gemini/mod.rs
  - crates/paladin-llm/src/http_status.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/mock.rs
  - crates/paladin-llm/src/openai/mod.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/redaction.rs
  - crates/paladin-llm/src/sampling.rs
  - crates/paladin-llm/src/services/commissary.rs
  - crates/paladin-llm/src/usage_reconcile.rs
  - crates/paladin-llm/src/window.rs
  - crates/paladin-notifications/src/lib.rs
  - crates/paladin-storage/src/cadence/contract_tests.rs
  - crates/paladin-storage/src/cadence/mod.rs
  - crates/paladin-storage/src/cadence/resilient.rs
  - crates/paladin-storage/src/cron.rs
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/run_trace/contract_tests.rs
  - crates/paladin-storage/src/run_trace/retention.rs
  - crates/paladin-storage/src/scheduler.rs
  - crates/paladin-storage/src/treasury/contract_tests.rs
  - crates/paladin-storage/src/treasury/notice_contract_tests.rs
  - crates/paladin-storage/src/waypoint/retention.rs
  - src/application/mod.rs
  - src/application/services/arsenal/in_process_arsenal.rs
  - src/application/services/arsenal/vault_tools.rs
  - src/application/services/assistant/doc_registry.rs
  - src/application/services/assistant/mod.rs
  - src/application/services/assistant/resolver.rs
  - src/application/services/assistant/service.rs
  - src/application/services/orchestration/mod.rs
  - src/application/services/paladin/middleware/context.rs
  - src/application/services/paladin/middleware/history.rs
  - src/application/services/paladin/middleware/mod.rs
  - src/application/services/paladin/middleware/summarization.rs
  - src/application/services/paladin/middleware/tool_protocol.rs
  - src/application/services/paladin/middleware/vault_recall.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/parley/adapter.rs
  - src/application/services/run/events.rs
  - src/application/services/run/inspector.rs
  - src/application/services/run/resolver.rs
  - src/application/services/run/schedule/admin.rs
  - src/application/services/run/schedule/service.rs
  - src/application/services/run/submission.rs
  - src/application/services/treasurer/mod.rs
  - src/config/agent_runtime.rs
  - src/core/mod.rs
  - src/infrastructure/cadence.rs
  - src/infrastructure/mod.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/telemetry/mod.rs
  - src/infrastructure/telemetry/persisting_sink.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/lib.rs
  - src/presets/mod.rs
findings:
  critical: 0
  warning: 3
  info: 35
  total: 38
status: issues_found
---

# Phase 55: Code Review Report

**Reviewed:** 2026-11-01
**Depth:** deep
**Files Reviewed:** 154 (plus the deleted `crates/paladin-content/src/services/content_filtering_service.rs`, read from `git show 7f2eabe93:<file>`)
**Diff range:** `7f2eabe93..HEAD` (HEAD `fd03df3b6`)
**Status:** issues_found

## Summary

This is the consolidated report of a multi-agent review: 14 reviewer groups plus completeness-gap reviews, with every finding checked by independent skeptics. Each upheld finding below was re-opened at its cited `file:line` during consolidation, and every quoted line still reads as quoted at HEAD.

No critical defect survived verification. The phase's main behaviour changes hold up:

- Panicking stubs are now `Err` returns.
- The `model_unpriced` halt is persisted through `persist_agent_halt`.
- The boot pricing warning is advisory and never refuses boot.
- The new stub-panic gate passes on the real tree.

The defects sit at the seams around those changes:

- **Coverage claims that are narrower than they read.** The boot pricing warning skips pinned older versions of stored assistants and does not say so (WR-01). The stub-panic gate exempts too much when `#[cfg(test)]` sits on a non-item (WR-03).
- **An upgrade note that leaves out a delivery change.** M-B-31 never tells webhook subscribers that a `failed`-only subscription now receives nothing for this case (WR-02).
- **Documentation drift from the Failed-to-Halted move.** Several docs promise a resume that never happens, or still list two halt reasons (IN-06 to IN-10).
- **Rustdoc link-sweep side effects** (plans 55-08, 55-10 and 55-11): over-long lines, code spans that name paths that do not exist, and resolvable links demoted to plain text (IN-18, IN-22, IN-29 to IN-35).

No finding involves a credential leak, a new `#[allow]`, a layering break, or a non-additive public-API change.

## Warnings

### WR-01: The boot pricing warning checks only each stored assistant's latest version, though pinned older versions still run, and the warning text does not list this gap

**File:** `src/infrastructure/web/run_api_wiring.rs:863` (the scan), `:805-813` (the warning text); `docs/src/architecture/treasurer.md:102-107`; `docs/src/getting-started/configuration.md:483-487`; `MIGRATION.md:128` (§11.8 step 6) and M-B-32
**Issue:** `scan_stored_agent_models_paged` reads only `repo.get_version(&item.assistant_id, item.latest)`. Older versions are still reachable by a metered principal:
- `POST /v1/runs` accepts `version: Some(v)` (`submission.rs:447`, `let use_latest = request.version.is_none() && resolved.source == AssistantSource::Stored;`).
- A schedule can pin a version (`run_schedule.rs:171`).
- `StoredAssistantResolver::resolve` serves any version from 1 to `latest`.

Admission then runs `admit_for_model` with the pinned version's `paladin.data.model`. The phase's own test seeds `stored-old` as v1 and `stored-a` as v2, and asserts that only `stored-a` is collected (line 2037 onward).

D-07 (as reconciled) requires the warning to name every class it cannot see "so an operator does not read silence as full coverage". The unchecked-class list in the warning text, treasurer.md, configuration.md and MIGRATION does not mention earlier versions.
**Failure scenario:** v1 of a stored agent-kind assistant uses model `m-old`, which has no price row, and v2 uses a priced model. Boot logs nothing about `m-old`. A client or schedule pinned to v1 is refused with `422 model_unpriced`, or halts `model_unpriced` at dispatch, while the operator believes every reachable model is priced.
**Evidence:**
```rust
// run_api_wiring.rs:863
match repo.get_version(&item.assistant_id, item.latest).await {
// run_api_wiring.rs:809-811
"... workflow graph-node models, runtime-provisioned agents and agent-kind assistants created \
 after boot are not checked here -- add a treasurer.pricing row for it"
```
**Fix:** Pick one.
- Walk every non-deleted version (`list_versions(id, page, cursor)`) and collect each agent-kind version's `model`.
- Or add the class to the unchecked list, for example `"..., agent-kind assistants created after boot and earlier (pinned) versions of stored assistants are not checked here"`. Update treasurer.md, the configuration.md bullet, MIGRATION §11.8 step 6 and M-B-32 to match, and add the needle to `unpriced_model_warning_line_names_the_model_and_the_gaps`.

**Verification:** 3/3 upheld at warning. Reproduce: the resolver serves any version, and the scan reads only `latest`. Context: the plan chose the latest-only scan, but reconciled D-07 binds the warning text to list every uncovered class. Impact: pinned runs are refused or halted with no boot signal.

### WR-02: M-B-31 never says that a webhook subscribed to `failed` but not `halted` now receives no delivery for this case

**File:** `MIGRATION.md:21` (M-B-31 "Required user action"); `CHANGELOG.md:22`; mechanism at `src/application/services/run/worker.rs:473-476` and `:1929`; documented example at `docs/src/api-reference/platform-api.md:64`
**Issue:** Before this phase, an agent-kind run whose model lost its price row at dispatch went through `persist_failure` and enqueued a `failed` webhook. It now enqueues `halted` (`enqueue_webhook_delivery(run, RunStatus::Halted, ...)`). Deliveries are filtered by the run's own subscription, and there is no implicit "all" default. The documented `POST /v1/runs` example subscribes to `["completed", "failed", "awaiting_input"]`, without `halted`.

M-B-31 tells clients only that one "must also match `halted` with this reason". That reads as a payload-matching change. A receiver that updates its matching code still receives nothing, because the delivery is never enqueued.
**Failure scenario:** A run or stored schedule whose webhook was copied from the documented example loses its notification for this case after the upgrade. This includes runs already queued at upgrade time and every future run of an existing schedule. The receiver gets no signal that anything changed.
**Evidence:**
```rust
// worker.rs:473-476
let webhook = run.webhook.as_ref()?;
if !webhook.events.contains(&kind) {
    return None;
}
// worker.rs:1929
self.enqueue_webhook_delivery(run, RunStatus::Halted, None, Some(&reason))
```
M-B-31: "A client that matched on `failed` for this case must also match `halted` with this reason."
**Fix:** Extend the M-B-31 action cell, the root CHANGELOG bullet, and optionally MIGRATION §11.8 with: "A run or schedule whose `webhook.events` subscribes to `failed` but not `halted` no longer receives any delivery for this case. Add `halted` to `webhook.events` (existing schedules must be updated) and match `halt_reason.reason == \"model_unpriced\"`."
**Verification:** Merged from two upheld findings, groups `halt` (3/3, warning) and `records` (3/3, info). Reproduce: the event-kind filter was traced end to end. Context: D-04 required MIGRATION to record the status change, and the effect on subscriptions is recorded nowhere. Impact: two of the six lenses rated it info, because subscriptions are opt-in. It is kept at warning because the upgrade note would lead an operator to the wrong remedy.

### WR-03: In the stub-panic gate, `#[cfg(test)]` on a field, variant, match arm, parameter or struct-literal field exempts the next braced item and hides reachable stubs

**File:** `scripts/check-no-stub-panics.sh:191-204` (`item_end`), `:218-219` (span assembly); claim at the header, line 20
**Issue:** `item_end()` assumes every `#[cfg(test)]` attaches to an item that ends at the first top-level `;` or `{...}`. It scans only for `{` and `;`, stepping over `,` and `}`. When the attribute sits on a non-item (a struct field, enum variant, match arm, function parameter or struct-literal field), the scan runs past the enclosing `}`. It then claims the next `{...}` block as the "item" and exempts all of it, which can be a whole production `impl` or `fn`.

Run in memory with the script's own `blank()` and `analyse()`, each of these returned no hits even though the stub is reachable in a non-test build:
- `pub struct S { #[cfg(test)] probe: u8, real: u8, } impl S { pub fn f(&self) { todo!() } }`
- `pub enum E { #[cfg(test)] Mock, Real, } pub fn f() { todo!() }`
- `#[cfg(test)] E::Mock => 0, E::Real => todo!(),`
- `pub fn f(#[cfg(test)] _x: u8, y: u8) -> u8 { todo!() }`
- `S { #[cfg(test)] probe: 0, real: unimplemented!(), }`

The header says the exemption covers "only the item or module block they attach to" (D-01: "brace-tracked after the attribute"). The harness has no fixture for any of these positions.
**Failure scenario:** A contributor adds a test-only struct field (a common Rust pattern) and leaves a `todo!()` in the following `impl`. The FIXES-01 gate passes, and a published crate ships a reachable panic. The real tree has no such pattern today: the only non-item target, the `thread_local!` at `crates/paladin-llm/src/cadence.rs:352`, gets a correct span. So this is a latent false negative in a CI gate.
**Evidence:**
```python
def item_end(b, j):
    k = skip_attrs(b, j)
    while k < len(b) and b[k] not in "{;":
        k += 1
```
**Fix:**
1. After `skip_attrs`, exempt only a real item head: an optional `pub(...)`, optional `async`/`unsafe`/`const`/`extern "..."`, then one of `fn|mod|impl|struct|enum|union|trait|type|const|static|use|extern|macro_rules|let`, or an `ident!` macro call. Otherwise exempt nothing.
2. Stop the header scan at `,` or at an unmatched `}`.
3. Add fire fixtures for the field, variant, match-arm and parameter positions.

**Verification:** 3/3 upheld. All three lenses reproduced the false negative in memory. All three rated it warning rather than the claimed critical, because no current tree pattern triggers it.

## Info

### IN-01: No test checks that boot emits the pricing warning, and the gate flags are hard-coded `true`

**File:** `src/infrastructure/web/run_api_wiring.rs:1185-1193`; test `boot_with_an_unpriced_reachable_model_under_an_allowance_still_succeeds` (line 2212)
**Issue:** The production call passes literal `true, true` for `treasurer_configured`/`has_ceiling`, so the table-test rows where either flag is false cover code production never runs. The real gate is the enclosing `if treasurer_config.allowance.is_empty()` else-arm (line 1152). The boot test asserts only that boot succeeds and that `handles.treasurer.is_some()`.
**Failure scenario:** Deleting the `report_unpriced_reachable_models(...)` call, or moving it outside the Treasurer branch, leaves every test green.
**Evidence:** `report_unpriced_reachable_models(&unpriced_reachable_models(true, true, &price_table, reachable.iter().map(String::as_str)), scan_stop);`
**Fix:** Pass `!treasurer_config.allowance.is_empty()` (or tenant/api-key non-emptiness) as `has_ceiling`. Then either factor the reachable-set assembly into a pure helper the boot test can assert on, or capture `PRICING_LOG_TARGET` logs in the boot test and assert that the model line is present.
**Verification:** 1/1 upheld (reproduce). The facts are correct, and only the "gating is meaningless" framing overstates.

### IN-02: Adjacent configuration.md text still calls WINDOWS row 67 "open"; the phase waived it

**File:** `docs/src/getting-started/configuration.md:481-482`
**Issue:** This phase moved `.planning/WINDOWS.md` row 67 from `open` to `waived`, and the new treasurer.md:107 paragraph calls the gap "accepted, documented". The bullet directly above the new boot-warning bullet still says "known, open gap".
**Failure scenario:** An operator reading the configuration reference sees the opposite status from the Treasurer guide and the register.
**Evidence:** configuration.md:481: "This is a known, open gap (`.planning/WINDOWS.md` row 67), not a guarantee". treasurer.md:107: "coverage is an accepted, documented gap (`.planning/WINDOWS.md` row 67)".
**Fix:** Change it to "This is an accepted, documented gap (`.planning/WINDOWS.md` row 67, waived): give every model a graph can reach a row."
**Verification:** 1/1 upheld. The diff confirms the waiver was made in this phase.

### IN-03: treasurer.md gives the wrong reason for skipping stored workflow graph-node models

**File:** `docs/src/architecture/treasurer.md:105`
**Issue:** The guide says graph-node models are unchecked because "the assistant resolver cannot list them". The scan does not use the resolver. It pages the repository directly, loads every stored assistant's latest definition, workflow ones included, and discards workflow definitions in `Ok(_) => {}` (run_api_wiring.rs:874). Only code-registered workflow graphs are truly unlistable.
**Failure scenario:** An operator reads a design choice as a technical limit.
**Evidence:** "The warning does not check workflow graph-node models (the assistant resolver cannot list them)". At run_api_wiring.rs:864: `Ok(Some(version)) if version.definition.kind == AssistantKind::Agent => {...}`, and at :874: `Ok(_) => {}`.
**Fix:** "(code-registered workflow graphs cannot be listed, and stored workflow definitions' node models are not parsed)", or extend the scan to stored workflow bodies.
**Verification:** 1/1 upheld (reproduce).

### IN-04: The scan-stop line is not documented anywhere operators look, and the boot reporter has no cap

**File:** `src/infrastructure/web/run_api_wiring.rs:893-897`, `:911-913`; `docs/src/architecture/treasurer.md:102-107`; `docs/src/getting-started/configuration.md:483-487`; `MIGRATION.md:128` (§11.8 step 6) and M-B-32
**Issue:** Under `paladin::pricing`, boot can log a second kind of line, meaning the stored-assistant scan stopped and the assistants after that point were not checked. None of treasurer.md, configuration.md, step 6 or M-B-32 mentions it. Step 6 says only to look for warnings "that name a model". The schedule-scan precedent does document its stop line (MIGRATION step 3). Also, unlike the settle-time warn-once set (capped at 256, T-38-07), the boot reporter has no cap on distinct names. The risk is low: assistant creation is admin-only.

The stop line being `warn` rather than D-14's `error` matches the locked plan (55-09-PLAN.md:211) and is not a defect.
**Failure scenario:** An operator sees the stop line, or no model lines after a stop, and reads that as full coverage.
**Evidence:** `"stored agent-kind assistant scan for unpriced models stopped at an unreadable page or version ({position}); assistants from there on were not checked -- inspect the assistants store directly"`. MIGRATION.md:128: "look in the boot log for warnings under the `paladin::pricing` target that name a model ...".
**Fix:** Add to step 6, M-B-32 and the treasurer.md "Boot warning" paragraph: "If the same target logs that the stored agent-kind assistant scan stopped at an unreadable page or version, the assistants after the named position were not checked; inspect the store and price their models by hand." Optionally cap the emitted model lines and add an "N more suppressed" line.
**Verification:** Merged from `wiring` (1/1) and `records` (1/1). The reproduce lens confirmed the stop line is reachable from `build_run_api` and that none of the phase's docs mention it.

### IN-05: The hermetic trace tests accept any terminal status, so they do not show the scripted port answered

**File:** `src/infrastructure/web/run_api_wiring.rs:3656`
**Issue:** `persisted_trace_records` waits for `run.status.is_terminal()`, then counts only `run_traces` rows. A `failed` run also leaves trace records, and the off case counts 0 either way.
**Failure scenario:** The `EnginePortSource` override (the point of TESTS-01) stops reaching the worker pool, and the run fails. Both `build_run_api_persists_*` tests still pass.
**Evidence:** `if run.is_some_and(|run| run.status.is_terminal()) { break; }`
**Fix:** After the wait, assert `RunStatus::Completed`, and optionally that the output is `"canned answer"`.
**Verification:** 1/1 upheld (reproduce).

### IN-06: platform-api.md says a `model_unpriced` run "resumes once an operator adds the row"; nothing resumes it

**File:** `docs/src/api-reference/platform-api.md:164-165`, `:803-804`
**Issue:** Phase 55 added both sentences. `persist_agent_halt` writes `final_waypoint_id: None`, moves the run Running -> Halted (terminal) and acks the queue. Nothing watches the price table or re-enqueues halted runs. `ParleyPortAdapter::resume_with` needs an `AwaitingInput` Waypoint, which an agent-kind run never has. The price table is attached once at boot (`.with_pricing(Arc::new(price_table))`, run_api_wiring.rs:1196), so adding a row means a config change and a restart. The same document says a halted run is terminal (line 173) and must be resubmitted (lines 191-193, 806-809).
**Failure scenario:** An operator adds the row, restarts, and waits for a run that stays `halted` forever.
**Evidence:** :164-165: "the run resumes once an operator adds the row." :173: "A halted run is terminal: it never returns to `queued` or `running`".
**Fix:** "...the model is not called; the run stays `halted`. Add a `treasurer.pricing` row for the model (it takes effect at the next boot), then submit a fresh `POST /v1/runs` (an agent-kind run has no checkpoint and is never resumed automatically)." Make the same correction in the ADR-0057 amendment.
**Verification:** Merged from `halt` (3/3, claimed warning) and `halt-consumers` (3/3, claimed warning), plus the platform-api half of a `records` finding (2/3). Reproduce rated both warning, and context and impact rated them info, because the same page documents resubmission correctly.

### IN-07: The HaltReason rustdoc and M-B-31 promise "resume" and a kept checkpoint for a variant that only agent-kind dispatch produces

**File:** `crates/paladin-core/src/platform/container/allowance.rs:252`, `:209-213`; `MIGRATION.md:21`
**Issue:** `ModelUnpriced` has a single producer: `derive_dispatch_budget` (worker.rs:1872-1873), called only on the agent-kind path. Its halt writes `final_waypoint_id: None` (worker.rs:1915), so only resubmission works. The variant doc says "resume or resubmit", and M-B-31 says "Treat `model_unpriced` as resumable ... then resume or resubmit the run". The enum-level doc was not updated and still says every halt happens "at a superstep boundary or an agent-loop cutoff" and that "the run keeps its last checkpoint". This is the published rustdoc of the type 0.11.1 adds the variant to.
**Failure scenario:** A client looks for a Halted Waypoint to fork and finds `final_waypoint_id: null`.
**Evidence:** :252: "The run halts resumably: add the price row, then resume or resubmit it." :212: "A halt is a resume point, not a failure: the run keeps its last checkpoint".
**Fix:** Line 252: "...add the price row, then resubmit it (`POST /v1/runs`); an agent-kind run has no checkpoint to resume from." Lines 209-213: widen to "...or at agent-kind dispatch" and qualify with "a graph run keeps its last checkpoint; an agent-kind run has none". Make the same change in M-B-31.
**Verification:** Merged from `halt` (1/1), `halt-consumers` (1/1) and the MIGRATION half of `records` (2/3 upheld). The context lens on the records finding refuted it, because platform-api.md defines "resume" for agent-kind runs as a fresh `POST`. The enum-level checkpoint claim is still wrong regardless.

### IN-08: The documented cause "(the row was removed after admission)" is not the only path to `model_unpriced`

**File:** `crates/paladin-core/src/platform/container/allowance.rs:250-251`; `docs/src/api-reference/platform-api.md:163`, `:802`
**Issue:** Admission checks pricing only when a ceiling applies. `derive_budget` returns `Ok(None)` before any price lookup (derive.rs:107-109). A run admitted while its principal had no ceiling, with a ceiling added (and a restart) while the run is still queued, reaches the pricing lookup for the first time at dispatch. It halts `model_unpriced` even though no row was ever removed.
**Failure scenario:** An operator who reads the cause searches for a deleted row that never existed.
**Evidence:** derive.rs:107-109: `let Some(evaluation) = self.evaluate(subject).await? else { return Ok(None); };`
**Fix:** Word the cause generally, for example "(for example the row was removed, or an allowance ceiling was configured for the principal, after the run was admitted)", in all three places.
**Verification:** 1/1 upheld (reproduce).

### IN-09: Rustdoc on the halt path still says an unpriced model records `Failed`, and still lists two halt reasons

**File:** `src/application/services/run/worker.rs:1600-1601` (`run_agent`), `:181` (`emit_agent_run_finished`); `src/application/services/run/events.rs:124-127`; `crates/paladin-core/src/platform/container/trace.rs:357-359`
**Issue:** Plan 55-02 changed the behaviour and updated `DispatchBudget` and `derive_dispatch_budget`, but these docs still describe the old state:
- The `run_agent` rustdoc says "records `Failed`", which contradicts the single `DispatchBudget::Halt` arm a few lines below it.
- `emit_agent_run_finished` lists "an unpriced model" as a separate exit from a halt.
- The public `TraceEvent::RunFinished.halt_reason` doc credits only "the engine's spend guard" and lists two reasons.
- The SSE `done` mapping doc (events.rs) lists two reasons.

Runtime is correct, because both paths go through `wire_json`.
**Failure scenario:** A contributor or client reading these docs does not expect `model_unpriced` on the trace or on `done`.
**Evidence:** worker.rs:1600-1601: "a model that lost its price row records `Failed` naming the model". trace.rs:357-358: "when the engine's spend guard halted it on a Treasurer reason (`allowance_exhausted` or `ledger_unavailable`,".
**Fix:** "records `Halted` with `model_unpriced` naming the model". Fold the unpriced case into "a dispatch-time halt" at :181. Add `model_unpriced` (with the run worker named as a second producer) to trace.rs and events.rs.
**Verification:** Merged from `halt` (worker.rs, 1/1) and `halt-consumers` (trace.rs/events.rs, 1/1).

### IN-10: M-B-31 has no rolling-upgrade or downgrade note; a 0.11.0 reader cannot decode a `model_unpriced` RunFinished trace record

**File:** `MIGRATION.md:21`; `crates/paladin-storage/src/run_trace/sqlite.rs:141`; `crates/paladin-core/src/platform/container/allowance.rs:239`; `src/application/services/run/events.rs:826`
**Issue:** The run-row decoders tolerate an unknown variant (run/sqlite.rs:320). The trace store does not: `row_to_record` maps any serde error to `RunTraceError::Serialization`, and `HaltReason` is internally tagged with no `#[serde(other)]` arm. The replay paginator stops on any read error (`_ => return false`). This is the first new on-disk `HaltReason` variant, so this phase makes the case reachable.
**Failure scenario:** During a rolling upgrade (0.11.0 API replicas next to 0.11.1 workers on a shared database), or after a patch downgrade, SSE replay on an affected thread stops early at that record, including for later runs on the thread.
**Evidence:** `serde_json::from_str(&record_json).map_err(|e| RunTraceError::Serialization { source: e })`; `#[serde(tag = "reason", rename_all = "snake_case")]`
**Fix:** Add to M-B-31: "Upgrade every reader (API replicas serving SSE replay) before any worker that can record `model_unpriced`; once such a halt is recorded, a 0.11.0 binary cannot replay that thread's trace."
**Verification:** 1/1 upheld (reproduce). `ModelUnpriced` is absent at `7f2eabe93`.

### IN-11: File-fetcher helpers return `None`, but every call site turns it into a made-up `0` / `(0, 0)`, and the stale placeholder comments remain

**File:** `crates/paladin-content/src/adapters/input/file_content_fetcher.rs:69-71`, `:82-84`, `:95-97`, `:105`
**Issue:** The new helper rustdoc ("Not implemented: always returns `None`") and the 55-BASE-LAYER-REVIEW row ("absence, not a made-up value") present the helpers as honest. All three callers then `unwrap_or(0)` / `unwrap_or((0, 0))`, so `fetch_content` returns a successful `ContentItem` with duration 0 or resolution 0x0. The "For now / In a real implementation" comments at the call sites were left in place.
**Failure scenario:** A consumer cannot tell "unknown" from "zero".
**Evidence:** `// For now, we'll use a placeholder duration` / `let duration = self.get_video_duration(path).unwrap_or(0);`; `let resolution = self.get_image_dimensions(path).unwrap_or((0, 0));`
**Fix:** Reword the call-site comments, the helper rustdoc and the review row to say that 0 / (0, 0) stands in for "unknown" because the helpers are not implemented.
**Verification:** 1/1 upheld (reproduce). The path is reachable with a `file://` URL to an existing .mp4, .mp3 or .png file.

### IN-12: A new test and rustdoc lock in `ItemsReordered` for a value change, and the test is named "no change"

**File:** `crates/paladin-core/src/base/service/collection_versioning_service.rs:439`, `:457-461`, `:1206-1218`
**Issue:** `detect_collection_changes` reports `ItemsReordered` whenever the counts match and the items differ, so `[a1, b2] -> [a1, b3]` is reported as a reorder that never happened (`ItemModified` exists for this). The branch predates the phase. Plan 55-05 added new rustdoc stating it as the contract, and a test named `equal_collections_report_no_change` that asserts `ItemsReordered` for two unequal, unreordered collections.
**Failure scenario:** A caller feeding `detected_changes` to `track_item_change` records "Items reordered" for a value edit.
**Evidence:** `let (service, id) = two_versions(items, vec![named("a", 1), named("b", 3)]); ... assert_eq!(differing.detected_changes, vec![CollectionChangeType::ItemsReordered]);`
**Fix:** If the behaviour stays frozen for the patch release, reword the rustdoc to "same count, contents or order differ" and rename the test (for example `same_length_differing_collections_report_items_reordered`). Otherwise report `ItemsReordered` only for a true permutation.
**Verification:** 2/3 upheld at info (claimed warning). Reproduce refuted it, because the branch predates the phase. Context and impact upheld it at info, because the phase's new rustdoc and test name freeze the mislabel.

### IN-13: Rustdoc says items are "retained pending the ADR-0060 (g) review" and, in the same breath, that the review is recorded

**File:** `crates/paladin-core/src/base/entity/field.rs:9-11`; `collection.rs:6-8`, `:23-25`; `base/component/element.rs:27-30`; `base/service/field_version_service.rs:102-104`; `base/service/collection_versioning_service.rs:107-109`
**Issue:** `base/mod.rs:14-19` says the review is done and every item is kept. The item docs still say "pending".
**Failure scenario:** A reader cannot tell whether the review is still outstanding.
**Evidence:** field.rs:9-11: "retained pending the ADR-0060 (g) review. The decision is recorded in `55-BASE-LAYER-REVIEW.md` (keep)."
**Fix:** "The ADR-0060 (g) review (`55-BASE-LAYER-REVIEW.md`) kept it; finishing or retiring it is a next-minor decision." Apply this at all six sites.
**Verification:** 1/1 upheld (reproduce).

### IN-14: paladin-core rustdoc names the review by bare filename, unlike base/mod.rs and the sibling crates

**File:** `crates/paladin-core/src/base/entity/field.rs:10`; `collection.rs:7`, `:24`; `element.rs:29`; `field_version_service.rs:104`; `collection_versioning_service.rs:109`
**Issue:** A docs.rs reader of `paladin-ai-core` cannot locate `55-BASE-LAYER-REVIEW.md`. `base/mod.rs` and `sqlite_content_repository.rs:662` give the full `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/` path.
**Failure scenario:** The pointer leads nowhere for a crate consumer.
**Evidence:** collection.rs:7: "the decision is recorded in `55-BASE-LAYER-REVIEW.md`".
**Fix:** Use the repository-relative path at all six sites, or point to the base module docs, which already carry it.
**Verification:** 1/1 upheld (reproduce).

### IN-15: No test covers the whitespace-only `expected_output` branch

**File:** `src/application/services/paladin/planning_service.rs:508`; tests at `:729`, `:748`, `:778`
**Issue:** The locked behaviour (55-06-PLAN.md:20) covers "empty or whitespace". The code uses `.trim().is_empty()`, but the three new tests cover only `""` and a non-empty value.
**Failure scenario:** If the check is later "simplified" to `.is_empty()`, the dangling `EXPECTED OUTPUT:    ` line comes back and every test still passes.
**Evidence:** `let prompt = if subtask.expected_output.trim().is_empty() {`
**Fix:** Add a test with `Subtask::new("1".into(), "Analyze".into(), "  \n\t ".into())`. Assert that the prompt contains neither `EXPECTED OUTPUT:` nor "focused on the expected output", and does contain "focused on the subtask".
**Verification:** 1/1 upheld (reproduce). The code is correct; only the test coverage is missing.

### IN-16: Field, module and MIGRATION docs say "empty" and "omitted", but blank values are dropped and the closing clause is reworded

**File:** `crates/paladin-core/src/platform/container/planning.rs:126-128`; `src/application/services/paladin/planning_service.rs:8-10`; `MIGRATION.md:25` (M-B-35)
**Issue:** The docs describe the empty case only, and the code goes further:
- `execute_subtask` drops whitespace-only values too.
- The module doc says a caller-built `Subtask` "that carries an `expected_output` still has it placed in its prompt".
- M-B-35 says "A caller-supplied expected output is rendered exactly as before". A whitespace-only value used to be rendered.
- M-B-35 says the closing clause is "omitted", but the code rewords it to "focused on the subtask".

Separately, the paladin-core field doc names the outer-layer `PlanningService`. It is a plain code span, so neither rustdoc nor the layering rule breaks.
**Failure scenario:** A caller who sets a blank `expected_output` sees it silently left out.
**Evidence:** planning.rs:126: "empty when none was specified". planning_service.rs:508: `if subtask.expected_output.trim().is_empty() {`
**Fix:** Module doc: "...whose `expected_output` is non-blank still has it placed in its prompt; an empty or whitespace-only value is treated as none." M-B-35: "...omits its `EXPECTED OUTPUT` line and its closing sentence points at the subtask instead when none (or only whitespace) is set. A non-blank caller-supplied expected output is rendered exactly as before."
**Verification:** Merged from `planning` (1/1) and `records` (1/1).

### IN-17: The new self-hosted-pair test mostly repeats the existing refusal test, and its doc claims more than it checks

**File:** `crates/paladin-llm/src/config/layers.rs:686-717` (against `:660`)
**Issue:** Beyond the value pin (`len() == 2` plus the two `contains` checks), `reasoning_wire_providers_are_the_self_hosted_pair` repeats the `SELF_HOSTED_PROVIDERS` resolve loop and the `acme` refusal from the line-660 test. Its doc says the two sites "cannot drift", but it never touches the `reasoning.rs` site. That guarantee comes from both sites sharing the constant, plus the named sibling test.
**Failure scenario:** None at runtime. A maintainer may over-trust the test.
**Evidence:** "read ONE constant, so the two sites cannot drift" / `let shared = crate::reasoning::SELF_HOSTED_PROVIDERS; assert_eq!(shared.len(), 2, ...)`
**Fix:** Keep the value pin and drop the repeated loop. Alternatively, call `reasoning::admit` with `EffortSupport::WireUnsupported` for each provider and assert that the hint names `llm.<provider>.reasoning_wire`.
**Verification:** 1/1 upheld (reproduce).

### IN-18: Rewritten intra-doc links push many doc lines far past the 100-column guideline

**File:** `crates/paladin-llm/src/cadence.rs:14` (and `fallback.rs`, `registry/mod.rs`, `provider_factory.rs`); `src/application/services/run/cancel.rs:2-4`, `:172`; `crates/paladin-ports/src/input/allowance_admission_port.rs:3`, `:19` (and about 57 lines in that set); `crates/paladin-ports/src/output/spend_scope.rs:93` (and about 75 lines across 24 port files); `crates/paladin-storage/src/scheduler.rs:37`; `crates/paladin-llm/src/services/commissary.rs:34`; `crates/paladin-storage/src/cadence/resilient.rs:39`; `src/application/services/arsenal/vault_tools.rs:35` (and 87 lines in the facade set)
**Issue:** Plans 55-08, 55-10 and 55-11 expanded short links into inline full-path links without reflowing the text. Line lengths at HEAD (measured) include:
- cadence.rs: 12 lines over 100 columns, up from 1.
- fallback.rs: 38, up from 4.
- cancel.rs lines 2/3/4/172: 137/139/138/164 columns.
- scheduler.rs:37: 214; commissary.rs:34: 221; resilient.rs:39: 208; vault_tools.rs:35: 193.

This is against rust.instructions.md: "Keep lines under 100 characters when possible". rustfmt does not wrap comments, so no gate catches it.
**Failure scenario:** None at runtime, because rendered output is unaffected. Source docs become hard to read and review.
**Evidence:** cadence.rs:14: `//! paced per hop (plan 43-06), so [`with_cadence`](crate::cadence::with_cadence) passes a port reporting`
**Fix:** Use reference-style link definitions at the end of each doc block (`//! [`X`]: path::to::X`) and re-wrap the touched paragraphs to 100 columns.
**Verification:** Merged from six upheld findings (`llm-gating`, `cancel-commander`, `doc-ports-a`, `doc-ports-b`, `doc-misc`, `doc-facade`), each 1/1. The line counts were reproduced against `7f2eabe93`.

### IN-19: The Phalanx pointer says no Phalanx error-path test exists, but several do

**File:** `crates/paladin-battalion/src/commander.rs:3921-3923`
**Issue:** These tests already cover exactly what the deleted TODO asked for (success/failure counts, preserved results, failure details):
- `phalanx_service.rs::test_partial_failures_with_absorb` (line 838)
- `phalanx_service.rs::test_phalanx_metrics_with_partial_failures` (line 1050)
- `tests/integration/battalion/phalanx_integration_test.rs::test_phalanx_partial_failures_continue_on_error` (line 355)

The wording came from 55-03-PLAN.
**Failure scenario:** A maintainer writes a duplicate or weaker Phalanx error test, which is the outcome threat T-55-10 was meant to prevent.
**Evidence:** "which builds a Formation; no Phalanx-specific error-path test exists (this test checks the happy path only)."
**Fix:** Name the real Phalanx coverage, or narrow the claim to "no Commander-routed Phalanx error-path test exists".
**Verification:** 3/3 upheld. All lenses rated it info (claimed warning).

### IN-20: Commander stub docs still describe failure behaviour that the new pointer says they do not test

**File:** `crates/paladin-battalion/src/commander.rs:3803`, `:3861`, `:3932-3934`, `:3948`
**Issue:** The phase appended "(this test checks the happy path only)" but kept the failure-oriented summaries and inline comments. `MockPaladinPort` never fails.
**Failure scenario:** A reader takes a happy-path test for error coverage.
**Evidence:** :3803: "Test that fail-fast error strategy stops execution on first failure". :3932: "Configure to continue on error for partial failure handling", above a run-timeout-only Aegis. :3948: "Phalanx with partial failures should complete".
**Fix:** Rewrite the summaries as happy-path descriptions, drop or correct the comments at :3861 and :3932, and change the message at :3948 to "Phalanx should complete".
**Verification:** 1/1 upheld (reproduce).

### IN-21: The registration poll ignores a run task that finishes early, so a dispatch error shows up as a 30 s timeout

**File:** `src/application/services/run/cancel_tests.rs:361-363`
**Issue:** The loop checks only `is_registered`. If `run_once` returns before registering (an `Err` from lease, resolve or waypoint lookup, or `Ok(false)`), it spins until the outer 30 s timeout. The real error is never shown.
**Failure scenario:** A regression shows up as "local_cancel_signals_token must finish within 30s" instead of a specific failure. Only diagnostics are lost; no assertion was weakened.
**Evidence:** `while !tokens.is_registered(&run_id).await { tokio::time::sleep(Duration::from_millis(5)).await; }`
**Fix:** Add `assert!(!run_task.is_finished(), "run_once finished before registering the run");` inside the loop, or await the task and show its `Result`.
**Verification:** 1/1 upheld (reproduce).

### IN-22: Link labels name an item or path other than the target

**File:** `src/application/services/run/cancel.rs:4` (and :2-3, :172); `src/application/services/paladin/middleware/mod.rs:18`
**Issue:** Two rewritten links have labels that do not match their targets:
- In cancel.rs, the label `RunSubmissionService::cancel` points at the trait method `RunSubmissionPort::cancel`.
- In middleware/mod.rs, the label `crate::engine::hooks::NodeInterceptor` names a path that does not exist in the facade crate (src/lib.rs has no `engine`), while the target is `paladin_battalion::engine::hooks::NodeInterceptor`.

**Failure scenario:** A reader copies a non-compiling path, or the text names one item while the link opens another.
**Evidence:** `//! [`RunSubmissionService::cancel`](paladin_ports::input::run_submission_port::RunSubmissionPort::cancel) consults ...`; `//! | | [`crate::engine::hooks::NodeInterceptor`](paladin_battalion::engine::hooks::NodeInterceptor) ...`
**Fix:** Make each label match its target, for example `[`RunSubmissionPort::cancel`]` and `[`NodeInterceptor`](paladin_battalion::engine::hooks::NodeInterceptor)`.
**Verification:** Merged from `cancel-commander` (1/1) and `doc-facade` (1/1).

### IN-23: A lexer desync (for example a raw C-string `cr#"..."#`) blanks the rest of the file instead of failing

**File:** `scripts/check-no-stub-panics.sh:99` (`RAW_OPEN`), `:148-158`
**Issue:** The raw branch fires only at an `r`/`b` with no identifier character before it, and `RAW_OPEN` is `(?:br|r)(#*)"`. In `cr#"..."#` (stable since Rust 1.77; MSRV 1.88) the `r` follows `c`, so the text is lexed as an ordinary string and the state goes out of sync. Reaching EOF inside a string, raw string or block comment is never reported, so every misparse fails open.
**Failure scenario:** `pub const A: &core::ffi::CStr = cr#"a " b"#;\npub fn f() { todo!() }` gives no hits (checked in memory).
**Evidence:** `and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_"))` / `RAW_OPEN = re.compile(r'(?:br|r)(#*)"')`
**Fix:** Make `RAW_OPEN` `(?:br|cr|r)(#*)"` with the boundary check at the prefix's first letter. Have `blank()` flag a literal or comment still open at EOF, and report it as a named `UNPARSEABLE` failure.
**Verification:** 3/3 upheld. Reproduce rated it warning, and context and impact rated it info (claimed warning), because no current file uses the form.

### IN-24: `#[cfg(test)] mod x;` resolution ignores inline-module nesting and can exempt a production file

**File:** `scripts/check-no-stub-panics.sh:221`, `:292`
**Issue:** The declaration is always resolved against `module_dir()` of the declaring file, even inside an inline `mod outer { ... }`, where rustc would load `outer/x.rs`. It also crosses crate roots when lib.rs and main.rs share `src/`. The header's claim that it "can only exempt a file that exists at that exact path" understates that the file can be production code.
**Failure scenario:** lib.rs has `pub mod util;` and `mod outer { #[cfg(test)] mod util; }`. Production `src/util.rs` is then treated as test-only. All 26 resolved test-only files in today's tree are correct.
**Evidence:** `decl = MOD_ITEM.match(blanked, skip_attrs(blanked, m.end()))` / `os.path.join(module_dir(path), name + ".rs"),`
**Fix:** Track inline `mod name {` nesting when collecting `test_mods`, or refuse to exempt a file that a non-cfg(test) `mod` declaration also reaches, and report the conflict.
**Verification:** 1/1 upheld (reproduce).

### IN-25: `publish = []` is treated as publishable

**File:** `scripts/check-no-stub-panics.sh:258-262`
**Issue:** Cargo treats `publish = []` as unpublishable, but the script skips a package only when `publish is False`.
**Failure scenario:** A package with `publish = []` would get a spurious, fail-closed finding. No manifest in the tree uses that form today.
**Evidence:** `publish = package.get("publish", True)` ... `if publish is False: continue`
**Fix:** `if publish is False or publish == []: continue` (after workspace resolution), plus a harness fixture.
**Verification:** 1/1 upheld (reproduce).

### IN-26: Unused `line_of` helper in the gate harness, and missing fixtures for several advertised forms

**File:** `tests/scripts/check-no-stub-panics_test.sh:75-79`
**Issue:** `line_of()` is never called (shellcheck SC2317). The harness is missing fixtures for these forms:
- Fire cases: `std::todo!()`; `#[cfg(any(test, feature = ...))]`, which must not be exempt; a member inheriting `publish.workspace = true` with no workspace publish key.
- Silent cases: a multi-line `#[cfg(test)]`; an `#[cfg(all(test, ...))] mod x;` declaration; `publish.workspace = true` under `[workspace.package] publish = false`.

The current logic handles all of them (checked in memory).
**Failure scenario:** A regression in any of these forms would go unnoticed.
**Evidence:** `line_of() { grep -nF -- "$2" "${FX}/$1" | head -n1 | cut -d: -f1; }`
**Fix:** Delete or use `line_of`, and add the fixtures listed above.
**Verification:** 1/1 upheld (reproduce).

### IN-27: The paladin-content CHANGELOG says the removed file held six `todo!()` calls; it held eleven

**File:** `crates/paladin-content/CHANGELOG.md:16`
**Issue:** At `7f2eabe93` the file had 11 `todo!()` calls: six in `filter_content_list` and five in its test's `ContentItem` literal. The consumer-impact claim (the file never compiled) is accurate.
**Failure scenario:** The release record has a wrong count.
**Evidence:** `git show 7f2eabe93:crates/paladin-content/src/services/content_filtering_service.rs | grep -c 'todo!()'` -> 11, against "holding six `todo!()` bodies".
**Fix:** "holding `todo!()` placeholders (six in `filter_content_list`, five in its test)".
**Verification:** 1/1 upheld (reproduce).

### IN-28: The B5 re-home note runs into the frozen recommendation and breaks the inventory's reconciliation convention

**File:** `.project/v0.11.1/01-carried-gaps-inventory.md:38`
**Issue:** The note is appended straight after "schema snapshot refresh" with no separator, and the cell still opens with a bold **in**. Row C10 (line 60) wraps its note as `*(Reconciled 2026-11-01, Phase 54, ...)*`. The note also has "owner the next minor", missing its colon.
**Failure scenario:** A skim of the Rec column shows B5 as in scope for v0.11.1.
**Evidence:** "schema snapshot refresh Re-homed 2026-11-01 (Phase 55, D-05): exhaustive enums at the patch gate; owner the next minor; ..."
**Fix:** `... schema snapshot refresh *(Re-homed 2026-11-01, Phase 55, D-05: exhaustive enums at the patch gate; owner: the next minor; 42-06 closed by this pointer; see REQUIREMENTS.md v2 Eval harness)*`
**Verification:** 1/1 upheld (reproduce).

### IN-29: The new lib.rs comment says paladin-core links carry an explicit path, but the same diff removed 17 explicit paths

**File:** `crates/paladin-ports/src/lib.rs:47-50`
**Issue:** The redundant-link fixes did the opposite of what the comment says for imported items (for example arsenal_port.rs:65, 238-241, 467-468; citadel_port.rs:67, 562-563; garrison_port.rs:378).
**Failure scenario:** A maintainer follows the comment, writes an explicit path to an imported item, and hits `rustdoc::redundant_explicit_links`, which both bars treat as an error.
**Evidence:** "a link to a higher layer is written as a plain code span and a link to `paladin-core` carries its explicit path."
**Fix:** "...a link to a `paladin-core` item carries its explicit path unless the item is already imported into scope (then the shorthand is required, since redundant explicit targets also fail the bars)."
**Verification:** 1/1 upheld (reproduce).

### IN-30: Demoted code spans keep paths that exist nowhere

**File:** `crates/paladin-ports/src/output/llm_port.rs:219`, `:221`; `embedding_port.rs:164`; `garrison_port.rs:194`; `sanctum_port.rs:207`, `:209`; `file_storage_port.rs:466`; `battalion_port.rs:380`; `paladin_port.rs:375`, `:379`
**Issue:** Turning higher-layer links into code spans was right, but the old path text was kept:
- `crate::application::ports` and `crate::infrastructure::adapters::*` resolve against `paladin-ports`, which declares only `input` and `output`. The adapters live in paladin-llm, paladin-memory, paladin-storage or the facade (`paladin::infrastructure::adapters::sanctum`).
- `paladin_battalion::battalion` and `paladin_battalion::paladin` do not exist. The Paladin execution service is `paladin::application::services::paladin::PaladinExecutionService`.

**Failure scenario:** A reader who follows a "Related Modules" or "See Also" entry finds nothing.
**Evidence:** llm_port.rs:219: `//! - Application Ports (`crate::application::ports`)`. battalion_port.rs:380: `//! - `paladin_battalion::battalion` - Battalion execution services (adapters)`
**Fix:** Point the ports entries at `[Application Ports](crate)` (or `crate::output`). Name the owning crate or module for each adapter entry, for example `paladin::infrastructure::adapters::sanctum` and `paladin_battalion` (`formation_service`, `phalanx_service`, ...).
**Verification:** Merged from four upheld findings (`doc-ports-a` llm_port and battalion_port; `doc-ports-b` paladin_port and sanctum_port), each 1/1.

### IN-31: The same "See Also" list mixes explicit and shorthand forms for items imported the same way

**File:** `crates/paladin-ports/src/output/citadel_port.rs:561-564`; `arsenal_port.rs:465-468`
**Issue:** Only the entries rustdoc flagged were converted, so each list now uses two styles for items all `use`d in scope (citadel_port.rs:277-278, arsenal_port.rs:251-253).
**Failure scenario:** None at runtime. The inconsistency obscures the convention.
**Evidence:** `/// - [`PaladinState`](paladin_core::platform::container::citadel::PaladinState) - ...` next to `/// - [`BattalionState`] - ...`
**Fix:** Convert the remaining entries to shorthand and re-run `make doc-check`.
**Verification:** 1/1 upheld (reproduce).

### IN-32: The SanctumPort trait doc was edited on its literal-`\n` line, but not added to the deferred record that embedding_port got

**File:** `crates/paladin-ports/src/output/sanctum_port.rs:583`; `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md:31`
**Issue:** The whole trait doc is one 6174-character `///` line with literal `\n///` text. It renders as one paragraph, and its three ```` ```rust,no_run ```` examples are never compiled. Plan 55-08 edited this line (the `GarrisonPort` link) and left the defect, just as it did for embedding_port.rs, but only embedding_port.rs is recorded. The defect predates the phase.
**Failure scenario:** A second instance of the defect, on a public port trait, is tracked nowhere.
**Evidence:** `awk 'NR==583{print length}'` -> 6174; deferred-items.md:31 names only `embedding_port.rs`.
**Fix:** Add a sanctum_port.rs:583 entry under "From plan 55-08" in deferred-items.md.
**Verification:** 1/1 upheld (reproduce).

### IN-33: The gemini and compat module links are demoted backwards: resolvable links became spans while an unresolvable one stays a link

**File:** `crates/paladin-llm/src/gemini/mod.rs:5-6`; `crates/paladin-llm/src/lib.rs:258`; `crates/paladin-llm/src/compat/types.rs:4`
**Issue:**
- `[`adapter`]` broke only because inner docs resolve in the parent scope. `[`adapter`](crate::gemini::adapter)` would resolve in every build that renders the doc, yet it was made a plain span.
- Likewise `[`super::engine`](crate::compat::engine)` in compat/types.rs would resolve wherever the file compiles (`compat/mod.rs:14`, `pub mod engine;`).
- Meanwhile `[`crate::compat::CompatEngine`]` (gemini/mod.rs:6) and `[`compat::CompatEngine`]` (lib.rs:258) stay links, though `compat` is gated on `any(kimi, qwen, grok, ollama, openai-compatible)`, which does not include `gemini`.

**Failure scenario:** `cargo doc -p paladin-llm --features gemini` now reports a `broken_intra_doc_links` warning that the removed crate-level `#![allow]` used to hide. The CI bars (default and all-features) do not exercise this feature set.
**Evidence:** gemini/mod.rs:6: ``//! [`crate::compat::CompatEngine`]. See `adapter`'s module-level``. types.rs:4: ``built on `super::engine` ``.
**Fix:** Restore `[`adapter`](crate::gemini::adapter)` and `[`super::engine`](crate::compat::engine)`, and make both CompatEngine references plain code spans.
**Verification:** Merged from `doc-misc` gemini (1/1) and compat/types (1/1).

### IN-34: The `execute_json_schema` link became a code span, though it has a resolvable trait-method target

**File:** `src/application/services/paladin/middleware/context.rs:281`
**Issue:** The method exists only via `impl StructuredExecutorPort for PaladinExecutionService`. The phase's own Deviation 2 handled nine such links with live trait-method targets, and `paladin_execution_service.rs:3694` links this same method. The 55-11 SUMMARY also misfiles it under "cfg(test) module targets".
**Failure scenario:** A navigable link is lost, and the treatment is inconsistent.
**Evidence:** `/// `PaladinExecutionService::execute_json_schema``
**Fix:** `/// [`PaladinExecutionService::execute_json_schema`](paladin_ports::output::structured_executor_port::StructuredExecutorPort::execute_json_schema)`
**Verification:** 1/1 upheld (reproduce).

### IN-35: Code-span replacements dropped the path that told the reader where a `pub(crate)` item lives

**File:** `src/application/services/paladin/middleware/summarization.rs:9`; `src/infrastructure/web/facade_provisioner.rs:6`
**Issue:** `super::super::paladin_execution_service::effective_history` became a bare `effective_history`, and `super::agent_host::build_agent` became a bare `build_agent`.
**Failure scenario:** The reader loses the pointer to the defining module. The meaning survives.
**Evidence:** `//! why `effective_history``; `//! `build_agent` path as config load, so`
**Fix:** `` `paladin_execution_service::effective_history` `` and `` `agent_host::build_agent` ``.
**Verification:** 1/1 upheld (reproduce).

## Dropped at consolidation

None. Every upheld finding was re-opened at its cited `file:line`, and its quoted text still reads as quoted at HEAD. Some duplicates were merged instead of dropped (WR-02, IN-04, IN-06, IN-07, IN-09, IN-16, IN-18, IN-22, IN-30, IN-33).

## Refuted during verification

- **A listed assistant whose latest version reads as missing is skipped without recording a stop** (`run_api_wiring.rs:874`): no shipped repository can return `Ok(None)` for a listed, non-deleted assistant's `latest`.
- **treasurer.md lists `model_unpriced` under TreasurerSpendGuard** (`treasurer.md:35`): the sentence names the worker at dispatch as the decision point, which matches the code.
- **No persistence round-trip test for a `model_unpriced` halt_reason** (`run/contract_tests.rs:646`): the serde form is pinned elsewhere, so the claimed silent loss would fail an existing test.
- **The capabilities table still marks file ingestion Implemented** (`content-processing.md:169`): in that guide "ingestion" means fetching from a source, and the phase made no doc problem worse.
- **The port docs describe `ingest_content` as storing content** (`content_input_port.rs:20`): the trait doc states the port contract, not one adapter's status.
- **Commented-out `mod` lines remain** (`paladin-content/src/services/mod.rs:8`): these predate the phase and are out of scope.
- **The fence does not stop a made-up parent id for `Field::new`** (`field_version_service.rs:105`): the rustdoc's claim is narrow (unversioned-field history), and the code makes it true.
- **The nil sentinel appears as an all-zero id in `track_item_change`** (`collection_versioning_service.rs:368`): this is the documented sentinel and was not introduced or worsened by the phase.
- **A third hard-coded copy of the self-hosted pair** (`provider_factory.rs:2400`): it predates the phase (commit 5edf3c7f3, 51-06).
- **Package discovery hardcodes `crates/*`** (`check-no-stub-panics.sh:246`): the workspace's `members = [".", "crates/*"]` matches exactly, so it cannot be reproduced on this tree.
- **`STUB_PANICS_ROOT` is an unannounced switch** (`check-no-stub-panics.sh:69`): the header announces it by name at lines 57-59.
- **A notification_port code span keeps a wrong `crate::` path** (`notification_port.rs:367`): the wrong path predates the phase, which only removed the brackets.
- **Module headings were escaped instead of linked** (`src/core/mod.rs:17`): escaping is a valid choice and not a defect.
- **Four "boot warning vs settle-time pricing key" findings** (`run_api_wiring.rs:805`, `treasurer.md:105`, `configuration.md:483`, `MIGRATION.md:128`): settle-time pricing by the served model, exact matching and fallback bypass all predate the phase. The new text makes no coverage claim that contradicts them.

## Review method

- **Reviewer groups.** Fourteen groups each reviewed one slice of the `7f2eabe93..HEAD` diff against 55-CONTEXT.md (including its Research reconciliation), the per-plan PLAN/SUMMARY files and the project rules:
  - `wiring`, `halt`, `halt-consumers`
  - `storage-content`, `base-layer`, `planning`
  - `llm-gating`, `cancel-commander`, `guard`, `records`
  - `doc-ports-a`, `doc-ports-b`, `doc-misc`, `doc-facade`
- **Completeness-gap reviews.** These swept for cross-slice gaps, for example boot-warning versus settle-time pricing.
- **Skeptic verification.** Every warning or critical candidate went to three independent skeptics: reproduce (trace the code path), context (check against locked decisions and recorded residuals) and impact (realistic consequence and severity). Every info candidate got one reproduce check. A finding is upheld on a majority vote, and severity follows the lens verdicts, which in several cases set it below the claimed level.
- **Consolidation.** The consolidation pass merged duplicates with a shared root cause, re-checked every cited line at HEAD (`fd03df3b6`), and wrote this report. It added no new findings.

---

_Reviewed: 2026-11-01_
_Reviewer: Claude (gsd-code-reviewer, consolidation pass)_
_Depth: deep_
