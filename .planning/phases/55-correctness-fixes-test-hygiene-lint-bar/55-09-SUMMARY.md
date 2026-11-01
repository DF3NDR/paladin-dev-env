---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 09
subsystem: run-api-wiring
tags: [treasurer, pricing, boot-warning, hermetic-tests, engine-port-seam, tests-01, fixes-06]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "55-02: HaltReason::ModelUnpriced and the removed DispatchBudget::Unpriced; Wave 1 gate: the two build_run_api_persists_* tests were the only failing tests"
provides:
  - "pub(crate) build_run_api_with_engine_port and EnginePortSource: the run engine's PaladinPort can be injected by in-crate tests only; the public build_run_api* functions delegate with no override"
  - "build_run_api_persists_run_traces_when_trace_persist_is_set and build_run_api_persists_no_run_traces_by_default pass with no network, no key and no serial group"
  - "a boot warning, one per unpriced model a metered principal can reach (code registry plus stored agent-kind assistants), logged under paladin::pricing, never refusing boot"
  - "Treasurer guide and configuration guide paragraphs naming the warning and the three classes it cannot check"
affects: [55-12, 55-13]

tech-stack:
  added: []
  patterns:
    - "A crate-private seam struct carries the test-only override beside an existing option so the delegating signature stays within clippy's argument limit"
    - "A boot advisory is a pure decision function plus a pure line renderer plus a thin infallible logger, with a read-only paged walk that reports a stop by position only"

key-files:
  created: []
  modified:
    - src/infrastructure/web/run_api_wiring.rs
    - docs/src/architecture/treasurer.md
    - docs/src/getting-started/configuration.md

key-decisions:
  - "PD-09-1 followed: the override rides in EnginePortSource beside cadence, not as an eighth parameter, so no lint attribute is needed"
  - "The stored-scan stop position is the last assistant read completely before the failure, so a failing get_version and a failing list page share one 'after assistant id X' / 'at start' sentence"
  - "Warning text is built from the model name alone, escaped with escape_debug; no tenant, key name or key value can reach it"

patterns-established:
  - "Hermetic run-API test: build through the seam with a scripted PaladinPort, register an executor the path never consults, and stop the pool through the ShutdownCoordinator before reading the store"

requirements-completed: [TESTS-01, FIXES-06]

duration: 11 min
completed: 2026-11-01
---

# Phase 55 Plan 09: hermetic run-API trace tests and the unpriced-model boot warning Summary

**The two `build_run_api_persists_*` tests now drive a real run through the production router and worker pool with a scripted `CannedAnswerPaladinPort` injected through a crate-private seam (no key, no network, no serial group), and server boot logs one `paladin::pricing` warning per unpriced model a metered principal can reach without ever refusing boot.**

Plan-start SHA: `bcd78e3d3b3fb818297109037128f23753ded06e`

## Performance

- **Duration:** 11 min (17:10:25Z to 17:21:00Z)
- **Tasks:** 3 of 3 (Task 1 is the tracer)
- **Files modified:** 3 (0 created): `src/infrastructure/web/run_api_wiring.rs` (+638 / -20 across two commits), two guides (+17)

## Accomplishments

- **Seam (D-08 as reconciled).** `build_run_api_with_cadence` is now a one-statement delegation to `pub(crate) async fn build_run_api_with_engine_port(.., engine_port: EnginePortSource)`. `EnginePortSource { cadence, paladin_port_override }` is `pub(crate)`; `None` calls `paladin_port_from_settings_with_cadence` exactly as before with the same error mapping. The only `paladin_port_override: Some(..)` constructions are inside `mod tests`. `build_run_api` and `build_run_api_with_cadence` keep their signatures and rustdoc; `src/bin/paladin-server.rs` is untouched.
- **Hermetic tests (TESTS-01).** `persisted_trace_records` builds through the seam with `CannedAnswerPaladinPort`, sets and removes no `OPENAI_API_KEY`, and names its registry executor `UnconsultedExecutor` (documented as never consulted on the agent-kind path). Both tests dropped their serial-group attribute.
- **Boot warning (FIXES-06, D-07 as reconciled).** `unpriced_reachable_models` (pure, sorted, de-duplicated, empty unless both flags are true), `unpriced_model_warning_line` (escaped model name, names the refusal, the not-refused workflow assistants and the three unchecked classes), `scan_stored_agent_models[_paged]` (read-only paged walk of `AssistantRepositoryPort::list` with `get_version(id, latest)`, never an error, stop reported by position only), `stored_scan_stop_line`, and `report_unpriced_reachable_models` (infallible, logs under `PRICING_LOG_TARGET`). It is called in the Treasurer branch after `price_table()` and before `Treasurer::new`.
- **Docs.** The Treasurer guide gains a "Boot warning" paragraph with one rendered line; the configuration guide gains one bullet under "A missing row".

## Task Commits

1. **Task 1 (tracer): seam and hermetic trace tests** - `65b61cce5` (test)
2. **Task 2: boot warning, pure functions, stored-assistant walk** - `173072c0c` (feat)
3. **Task 3: guide paragraphs** - `3c0d189f2` (docs)

Tracer feedback gate (auto mode active): the tracer's `<verify>` was run end to end on the committed Task 1 tree before expanding and passed. Tracer verified end-to-end, expanding.

## WINDOWS row 73 reproduction (before)

`env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` at the plan-start SHA:

```text
failures:
    infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default
    infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1483 filtered out; finished in 20.74s
```

Both panicked at the same assertion: `the run never reached a terminal status` (the real model call left the process and the proxy refused it).

## No-network run (TESTS-01)

After the seam, with every proxy variable and `OPENAI_API_KEY` unset:

```text
$ env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy -u OPENAI_API_KEY cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set ... ok
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1488 filtered out; finished in 1.36s
```

The two tests went from a 20.74 s failure to a 1.36 s pass. The whole `infrastructure::web::run_api_wiring` module (network unchanged, the other tests still set the key under their serial group): `test result: ok. 44 passed; 0 failed`.

## Watched failing first

- Task 1: after the test edits and before the seam existed, `cargo test ... build_run_api_persists` failed to compile (`cannot find struct EnginePortSource`, `cannot find function build_run_api_with_engine_port`).
- Task 2: after the four tests (plus one boot test) were written and before the functions existed, `cargo test ... unpriced_reachable` failed to compile (`cannot find type PriceTable`, `StoredAgentModelScan`, and the missing functions). The four tests then went green: `unpriced_reachable_models_table`, `unpriced_model_warning_line_names_the_model_and_the_gaps`, `stored_agent_model_scan_reads_agent_kind_models_only`, `stored_agent_model_scan_stops_at_an_unreadable_page`.

## One rendered warning line

From `unpriced_model_warning_line("example-model")`, quoted verbatim in the Treasurer guide:

```text
model `example-model` is reachable by a metered principal but has no treasurer.pricing row: agent-kind calls on it are refused at admission (422 model_unpriced) or halted at dispatch (model_unpriced); workflow assistants are not refused; workflow graph-node models, runtime-provisioned agents and agent-kind assistants created after boot are not checked here -- add a treasurer.pricing row for it
```

## Acceptance and verification results

- `grep -c 'pub(crate) async fn build_run_api_with_engine_port'` prints 1 (PASS); the one `paladin_port_override: Some(` is at line 3100-ish, after `mod tests {` (PASS, this was re-checked after Task 2 moved lines).
- Serial-group check on the two tests prints 0; `OPENAI_API_KEY` inside `persisted_trace_records` prints 0; `FailingExecutor` there prints 0 and `UnconsultedExecutor` is named (PASS).
- `git diff bcd78e3d3 -- src/bin crates` is empty: `paladin-server.rs`, `paladin-llm` and `paladin-web` are unchanged (PASS).
- `target: PRICING_LOG_TARGET` appears only inside `report_unpriced_reachable_models` (returns `()`); line order is `.price_table()` (1173) < `report_unpriced_reachable_models(` (1185) < `Treasurer::new(` (1194) (PASS).
- `cargo fmt --check` clean; `cargo clippy -p paladin-ai --features web-server --lib --tests --offline -- -D warnings` and the `--lib` variant exit 0 (PASS).
- `make check-doc-config` (161 YAML blocks, 0 failed) and `make check-version-strings` (OK) exit 0 (PASS).
- `make api-surface` reads `API surface unchanged` (4325 items, nightly-2026-09-20) (PASS). Plan 55-13 still owns the phase-head semver and surface evidence.
- Every pre-existing Treasurer boot test passes, and the new `boot_with_an_unpriced_reachable_model_under_an_allowance_still_succeeds` boots through the seam with a registered agent on a model that has no price row (B4: boot never refuses).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan's guide anchors were stale; the "Two limits" passage is in `configuration.md`, not `treasurer.md`**
- **Found during:** Task 3
- **Issue:** the plan told Task 3 to edit "Two limits to know about" in `treasurer.md` (~l.588-596), but that file has 195 lines and no such heading. "Two limits to know about" and "A missing row" are both in `docs/src/getting-started/configuration.md`; `treasurer.md` carries the pricing prose under "Pricing as the cost source".
- **Fix:** the full operator contract (log target, boot continues, workflow assistants not refused, all three unchecked classes, the WINDOWS row 67 gap note and the verbatim `example-model` line) went into `treasurer.md`'s "Pricing as the cost source" section; the one new bullet went under "A missing row" in `configuration.md` with the same three classes and a link to the Treasurer guide. The two existing "A missing row" bullets and the "Two limits" passage are byte-unchanged. Every acceptance criterion and the `<verify>` greps still hold.
- **Files modified:** `docs/src/architecture/treasurer.md`, `docs/src/getting-started/configuration.md`
- **Commit:** `3c0d189f2`

**2. [Rule 2 - Missing coverage] Added one boot test beyond the four in the plan**
- **Found during:** Task 2
- **Issue:** the plan's acceptance says boot still succeeds with an unpriced reachable model under allowances, but no existing Treasurer boot test registers an agent on an unpriced model (the shared fixture prices `gpt-4`).
- **Fix:** `boot_with_an_unpriced_reachable_model_under_an_allowance_still_succeeds` registers an agent on `no-price-row-model`, builds through the seam with the scripted port, and asserts the Treasurer is present. It needs a second throwaway executor (`UnconsultedExecutorForBoot`) because `AgentRegistry::insert` requires one.
- **Files modified:** `src/infrastructure/web/run_api_wiring.rs`
- **Commit:** `173072c0c`

**3. Execution detail (stored-scan stop position):** the plan said the stop "records the cursor position". The scan records the last assistant read completely before the failure (`Option<Option<AssistantId>>`, `None` inside means the first page). For a failing `list` page this equals the cursor that page was requested with, and it gives a failing `get_version` the same position-only sentence. The tests assert both the first-page and the later-page shapes.

**Total deviations:** 2 auto-fixed (Rule 3 documentation anchor, Rule 2 added test) plus one position-recording detail. No public API change, no effect on production boot behavior beyond the advisory log lines.

## Known Stubs

None. `CannedAnswerPaladinPort`, `UnconsultedExecutor`, `UnconsultedExecutorForBoot` and `FailsSecondListAssistants` are test doubles inside `#[cfg(test)]`, not stubs in shipped code.

## Issues Encountered

- Disk stayed at about 9.6 GB free throughout, so no `target/` cleanup was needed. All cargo runs used `CARGO_INCREMENTAL=0` and `--offline`.
- A throwaway `println!` was used once to capture the rendered line for the guide; the file was restored from a scratchpad copy before the Task 2 commit was made, and it is not in any commit.

## Threat Flags

None beyond the plan's register. T-55-20 (egress from a unit test) mitigated by the scripted port and the recorded no-network run. T-55-21 and T-55-24 mitigated by a model-name-only, `escape_debug`-escaped line with a content test. T-55-22 mitigated by an infallible reporter and a never-erroring walk that runs only when a Treasurer is built. T-55-23 mitigated by the `pub(crate)` seam with `Some` built only in `mod tests` (acceptance check). The boot warning logs model names and uncovered classes only, never a key value or tenant id.

## Next Phase Readiness

- Plan 55-13 can disposition WINDOWS row 73 (`fixed`) on the No-network run above and row 67 (`waived`) on the Treasurer guide's documented graph-node gap, and re-run `make api-surface` and the manual credential-handling review at the phase head.
- Plan 55-12's CHANGELOG entries need no entry from this plan: no public item was added or changed.

## Self-Check: PASSED

- Files exist: `src/infrastructure/web/run_api_wiring.rs`, `docs/src/architecture/treasurer.md`, `docs/src/getting-started/configuration.md`.
- Commits `65b61cce5`, `173072c0c` and `3c0d189f2` exist on `claude/laughing-dirac-e0h2ax`.
- All three tasks' acceptance criteria re-run green on the final tree.
