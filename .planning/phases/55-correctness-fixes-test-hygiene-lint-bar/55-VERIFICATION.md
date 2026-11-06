---
phase: 55-correctness-fixes-test-hygiene-lint-bar
verified: 2026-11-02T01:00:58Z
status: passed
score: 5/5 roadmap success criteria verified (13/13 requirement IDs accounted for; FIXES-04 re-homed by recorded decision D-05)
behavior_unverified: 0
overrides_applied: 0
re_verification: false
deferred:
  - truth: "An eval scenario can assert `run_status: cancelled` (FIXES-04)"
    addressed_in: "Next minor (first phase), outside milestone v0.11.1"
    evidence: "ROADMAP SC2 carries the dated 'Reconciled 2026-11-01, Phase 55, D-05' note; REQUIREMENTS.md v1 line 165 is unchecked with the re-home note, v2 '### Eval harness' FIXES-04 entry names both semver lints and the owner, the traceability row reads 'Re-homed (next minor)'; RunStatusValue (scenario.rs:578) and Assertion (scenario.rs:603) were confirmed exhaustive, so the variant is a major change at the patch gate"
---

# Phase 55: Correctness Fixes, Test Hygiene & Lint Bar Verification Report

**Phase Goal:** Nothing in a published crate panics from an unfinished stub or misreports a status, the unfinished content/base-layer code is fenced with a recorded decision, and the test suite and lint bar are hermetic, non-flaky and warning-free, so the later phases and the release run on a clean baseline.
**Verified:** 2026-11-02T01:00:58Z at HEAD `e332ecc3d` (branch `claude/laughing-dirac-e0h2ax`)
**Status:** passed (with advisory warnings below)
**Re-verification:** No, this is the initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | No `todo!()`/`unimplemented!()` is reachable outside `#[cfg(test)]` in a published crate. A gate holds this and fails on a planted stub. The panicking stubs return a typed error or are gone. Every kept-but-unfinished base-layer/content item has a recorded keep/fence/remove decision and refuses cleanly, with rustdoc saying so. | VERIFIED | `scripts/check-no-stub-panics.sh` on the real tree prints `OK: scanned 624 file(s) (19 test-only), 0 finding(s)`, exit 0. It is wired at `Makefile:272` (a `check-gates` prerequisite) and at `.github/workflows/ci.yml:189`. The harness passes 88 assertions. My own planted `pub fn planted() -> u8 { todo!() }` in a scratch copy (via `STUB_PANICS_ROOT`) gave exit 1 with `crates/paladin-storage/src/lib.rs:122`. A raw C-string plus a match-arm `unimplemented!()` probe also fired. A naive grep over `crates/*/src` and `src` found about 50 hits; every one I sampled sits in a `#[cfg(test)] mod tests` block or in `engine/test_support.rs`, which is `#[cfg(test)]` at `engine/mod.rs:90`. The 20 storage bodies return `Err(unfinished_content_list(..))` (`sqlite_content_repository.rs:663+`, `mysql_content_repository.rs:640+`). Both refusal table tests pass; mysql was run from a cached `--features mysql` binary built after the last storage change. `content_filtering_service.rs` is deleted and has no `mod` line. `55-BASE-LAYER-REVIEW.md` has one row for each of the 8 items. Fences checked in code: `field_version_service.rs:507-510` (`EntityNotFound`), `collection_versioning_service.rs:432/434` (`Uuid::nil()` sentinel), `planning_service.rs:418-423` (`String::new()`), `file_content_fetcher.rs:16,194` (`Err`). Rustdoc says "Unfinished scaffolding" on `Field`, `Item`/`CollectionType`, `element`, `FieldVersionService` and `CollectionVersionService`. The named fence tests pass. |
| 2 | `DispatchBudget::Unpriced` on an agent-kind run records `Halted`. `run_status: cancelled` is re-homed (D-05). The `claude-opus-4-8` `reasoning` flag matches the probe record. Boot warns once per unpriced model a metered principal can reach and does not refuse. | VERIFIED | `worker.rs:1872-1873` maps `AdmissionError::ModelUnpriced` to `DispatchBudget::Halt(HaltReason::ModelUnpriced{model})`. The old `Unpriced` arm that mapped to `Failed` is gone. `HaltReason` is `#[non_exhaustive]` (`allowance.rs:240`). `agent_kind_run_whose_model_became_unpriced_halts_without_calling_the_llm` passes and asserts `RunStatus::Halted`, `error.is_none()`, SSE `done` with `status: halted`, the `halt_reason` wire JSON, the webhook kind `Halted`, and 0 LLM calls. The FIXES-05 flag stays `reasoning: false`, which is correct: 50.1 probe row 1's `reasoning text` column reads `absent`, and `ModelProfile.reasoning` (`model_profile.rs:963`) is defined as "the default call returned reasoning text". The test `anthropic_opus_4_8_seed_matches_the_transcript` pins it. FIXES-06: the boot path at `run_api_wiring.rs:1226-1243` gathers code-registry models plus every agent-kind version of the stored assistants. It filters with the dispatch lookup (`PriceTable::row`), de-duplicates through a `BTreeSet`, and logs one `warn!` per model under `paladin::pricing`. `unpriced_reachable_models_table` and `boot_with_an_unpriced_reachable_model_under_an_allowance_still_succeeds` pass. The `run_status: cancelled` clause is re-homed (see Deferred). |
| 3 | `cargo test` passes with no network, including the two `build_run_api_persists_*` tests against a scripted port. `local_cancel_signals_token` waits on a registration signal and survives repeated runs under parallel load. WINDOWS row 73 is dispositioned. | VERIFIED | Both tests pass inside `unshare -rn` (no network namespace at all). The whole facade lib binary passes 1495/1495 with only loopback up and every proxy variable unset. So do 11 crate lib binaries: eval 56, ports 299, llm 836, memory 150, core 751, storage 541, herald 60, content 96, battalion 896, web 329 and notifications 0. Both tests inject `CannedAnswerPaladinPort` through the `pub(crate)` `build_run_api_with_engine_port` / `EnginePortSource` seam (`run_api_wiring.rs:1017,1029`), and the public signatures are unchanged. `cancel.rs:221-224` adds a `#[cfg(test)] is_registered`, and the test polls it inside a 30 s timeout (`cancel_tests.rs`). The fixed sleep is gone. I ran the test 10 times inside the full 264-test `application::services::run::` filter: 10/10 ok. `WINDOWS.md` row 73 reads `fixed` and cites the tests and the seam. |
| 4 | Single-feature `cargo clippy --tests` on `paladin-llm` is warning-free. The Phase 51 lint leftovers are cleared. `cargo doc` is warning-free under both bars with the eight crate-level allows removed. `doc-coverage-report.md` is regenerated. | VERIFIED | `grep -rnE '(allow\|expect)\(rustdoc' crates/*/src src` finds 0, and `Cargo.toml` has no rustdoc lint table. `cargo clippy -p paladin-llm --tests --no-default-features --features mock -- -D warnings` and the same with `--features ollama` finish clean; the cache replays no warnings. `paladin-llm` has had no change since `8ea9f1357`, so the evidence file's 13-combination matrix still holds. The only Rust change after the evidence head is `run_api_wiring.rs`, so I re-ran both rustdoc bars on `paladin-ai` at HEAD. `cargo doc -p paladin-ai --no-deps` gave exit 0 with 0 warnings. `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` gave exit 0. The `NODE_CACHE_KEY_VERSION` link is fully qualified (`cache_key.rs:44`). `doc-coverage-report.md` carries "Regenerated 2026-11 (Phase 55, CIREL-08)" and the commit SHA. ADR-0033 is amended at line 244. |
| 5 | The self-hosted provider pair is defined once. The `commander.rs` "TODO: Enhance MockPaladinPort" stubs are deleted or point at `commander_error_paths_test.rs`. | VERIFIED | There is one `pub(crate) const SELF_HOSTED_PROVIDERS` (`reasoning.rs:263`), imported by `config/layers.rs:58`, and `REASONING_WIRE_PROVIDERS` is gone. `Enhance MockPaladinPort` has 0 hits. The four tests carry a `/// Error-path coverage: tests/integration/commander_error_paths_test.rs <test>` pointer (`commander.rs:3805,3836,3873,3921`). The four named tests exist in that file (lines 53, 92, 153, 218). |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | FIXES-04 `RunStatusValue::Cancelled` / `run_status: cancelled` | Next minor (first phase) | The D-05 decision is recorded and honoured. Every artifact the decision names exists: rustdoc on `RunStatusValue` (scenario.rs:566-572) and on `Assertion::RunStatus` (:651); the regenerated `docs/schemas/eval-scenario.schema.json` description; the `eval-harness.md:77` row; REQUIREMENTS v1 note, v2 entry and traceability; ROADMAP SC2 note; inventory B5 (`.project/v0.11.1/01-carried-gaps-inventory.md:38`); and `deferred-items.md`. I confirmed in code that both enums are exhaustive. No `paladin-eval` enum changed, and semver reads 12/12 green per the evidence file. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/check-no-stub-panics.sh` + `tests/scripts/check-no-stub-panics_test.sh` | Offline zero-exemption gate and harness | VERIFIED | Real run is OK, harness passes 88/88, my planted probe fired, and the gate is wired into make and CI |
| `crates/paladin-storage/src/{sqlite,mysql}_content_repository.rs` | 20 typed refusals | VERIFIED | Code read; both table tests pass |
| `crates/paladin-core/src/platform/container/allowance.rs` | `HaltReason::ModelUnpriced` | VERIFIED | Additive on a non-exhaustive enum; serde, `as_str` and herald tests are present |
| `src/application/services/run/worker.rs` | Unpriced leads to Halt | VERIFIED | `:1872-1873` |
| `src/infrastructure/web/run_api_wiring.rs` | Hermetic seam + boot warning | VERIFIED | `pub(crate)` seam, pure function, scan, reporter, call site `:1226-1243` |
| `src/application/services/run/cancel.rs` | `is_registered` inspector | VERIFIED | `#[cfg(test)] pub(crate)`, so no public surface |
| `.planning/phases/55-.../55-BASE-LAYER-REVIEW.md` | One recorded decision per item | VERIFIED | 8 rows with design intent, state, decision and evidence; the decisions match the code |
| `docs/src/appendix/doc-coverage-report.md` | Regenerated measurement | VERIFIED | Dated banner, commit, both bar commands |
| `.planning/phases/55-.../55-CORRECTNESS-EVIDENCE.md` | Prove-then-lock record | VERIFIED | Present. Its gate table is at `8202da48c`, and later review-fix commits touched only the gate scripts, `run_api_wiring.rs` and docs. I re-checked the affected bars at HEAD (above). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Makefile check-gates` | `check-no-stub-panics.sh` | prerequisite target | WIRED | `Makefile:228-230,272` |
| CI gate-scripts job | `check-no-stub-panics.sh` | step | WIRED | `ci.yml:189` |
| `derive_dispatch_budget` | `persist_agent_halt` / SSE / webhook | `DispatchBudget::Halt` arm `worker.rs:1679` | WIRED | Proven by the passing regression test (row, SSE and webhook asserted) |
| `build_run_api_with_cadence` | `build_run_api_with_engine_port` | delegate with `paladin_port_override: None` | WIRED | Production cannot select a scripted port; `Some(..)` appears only in `mod tests` |
| Boot (allowance branch) | `report_unpriced_reachable_models` | call at `run_api_wiring.rs:1235` | WIRED | Runs only inside the Treasurer-plus-allowance branch, where `true, true` is correct |
| `config/layers.rs` | `reasoning::SELF_HOSTED_PROVIDERS` | `use` at `:58` | WIRED | Single definition |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate passes the real tree | `./scripts/check-no-stub-panics.sh` | `OK: scanned 624 file(s) (19 test-only), 0 finding(s)` | PASS |
| Gate fails on a planted stub | `STUB_PANICS_ROOT=<scratch> ./scripts/check-no-stub-panics.sh` | `FOUND ... lib.rs:122`, exit 1 | PASS |
| Hermetic run-API tests with no network | `unshare -rn <facade lib test bin> build_run_api_persists` | 2 passed | PASS |
| Whole facade lib suite with no network | `unshare -rn` + loopback up + proxy unset | 1495 passed, 0 failed | PASS |
| Crate lib suites with no network | same, 11 binaries | all `ok`, 0 failed | PASS |
| Unpriced leads to Halted | `... became_unpriced` | ok | PASS |
| Cancel test under load | 10 runs of the `application::services::run::` filter (264 tests) | 10/10 ok | PASS |
| Fences | named core, content and storage (sqlite + mysql) tests | all ok | PASS |
| llm single-feature clippy | `cargo clippy -p paladin-llm --tests --no-default-features --features {mock,ollama} -- -D warnings` | clean | PASS |
| Facade rustdoc, both bars at HEAD | `cargo doc -p paladin-ai --no-deps`; `RUSTDOCFLAGS=-D warnings ... --all-features` | exit 0, 0 warnings | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` is declared by this phase. Step 7c: not applicable. The gate harness above serves this role.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|------------|--------|----------|
| FIXES-01 | 55-01, 55-12, 55-13 | SATISFIED | Gate is wired and proven both ways; 20 typed refusals; dead file deleted |
| FIXES-02 | 55-05, 55-06, 55-12, 55-13 | SATISFIED | 8-row review; fences in code with tests; rustdoc on each item; todo closed (`todos/completed/...`, `resolves_phase: 55`) |
| FIXES-03 | 55-02, 55-12, 55-13 | SATISFIED | Local mapping to `Halted` + `model_unpriced`; treasurer.md (4 mentions), platform-api.md (8), openapi.json, M-B-31 |
| FIXES-04 | 55-07, 55-13 | RE-HOMED (D-05 recorded and honoured) | All re-home artifacts are present; exhaustiveness confirmed; traceability reads "Re-homed (next minor)"; v1 checkbox left unchecked, which is correct |
| FIXES-05 | 55-04, 55-13 | SATISFIED (verified-not-wrong) | The probe column reads `absent` and the field contract is "default call"; the flag stays `false`; comment corrected; row pinned by test |
| FIXES-06 | 55-09, 55-12, 55-13 | SATISFIED | Boot warning per unpriced reachable model; never refuses (tested); WINDOWS row 67 waived with reason |
| TESTS-01 | 55-09, 55-13 | SATISFIED | Passes under `unshare -rn`; row 73 `fixed` |
| TESTS-02 | 55-03, 55-13 | SATISFIED | Registration poll; 10/10 under load (the evidence file records 20) |
| TESTS-03 | 55-04, 55-13 | SATISFIED | 13-combination matrix (evidence); spot-checked 2 at HEAD; no new `allow`/`expect` |
| TESTS-04 | 55-04, 55-13 | SATISFIED | Link fixed; default clippy and doc clean |
| TESTS-05 | 55-04, 55-13 | SATISFIED | Single constant; 51 IN-01 closed as reproduced and fixed |
| TESTS-06 | 55-03, 55-13 | SATISFIED | Pointers in place; TODO blocks gone |
| CIREL-08 | 55-08, 55-10, 55-11, 55-13 | SATISFIED | 0 rustdoc allows; both bars green; report regenerated; ADR-0033 amended |

No orphaned requirements. All 13 IDs mapped to Phase 55 in REQUIREMENTS.md appear in at least one plan's `requirements:` field.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (154 files changed since `7f2eabe93`) | - | `TBD`/`FIXME`/`XXX` debt markers | none | Only `ci.yml:216,219`, a guard that greps MIGRATION.md for `TBD`, not a debt marker |
| `src/application/services/run/worker.rs` | 1600-1601 | Stale rustdoc: "a model that lost its price row records `Failed`" (REVIEW IN-09, not fixed) | WARNING | Runtime is correct (`Halted`). The `run_agent` doc misstates the status. |
| `crates/paladin-core/src/platform/container/trace.rs` | 357-359 | Public `RunFinished.halt_reason` rustdoc lists only `allowance_exhausted`/`ledger_unavailable` (IN-09) | WARNING | Published rustdoc omits the new `model_unpriced` producer |
| `docs/src/api-reference/platform-api.md`; `allowance.rs`; `MIGRATION.md` M-B-31 | 804; 252; 21 | Say "resumes once an operator adds the row" / "resume or resubmit", but an agent-kind halt has no checkpoint and nothing resumes it (IN-06/IN-07) | WARNING | Operator-facing wording contradicts "a halted run is terminal" on the same page |
| `crates/paladin-content/src/adapters/input/file_content_fetcher.rs` | 69-71, 82-84, 95-97 | `// For now, we'll use a placeholder ...` + `unwrap_or(0)` / `unwrap_or((0, 0))` (IN-11) | WARNING | `fetch_content` reports duration 0 / resolution 0x0 for "unknown". This predates the phase, but `55-BASE-LAYER-REVIEW.md` says "absence, not a made-up value", which overstates the fence |
| `src/infrastructure/web/run_api_wiring.rs` | ~3953 | `persisted_trace_records` waits for any terminal status (IN-05) | INFO | Hermeticity is proven, but the tests do not assert `Completed`, so they would not catch the seam no longer reaching the pool |
| `crates/paladin-storage/src/mysql_content_repository.rs` | 809 | mysql refusal test is feature-gated and `ci.yml` has no `mysql` feature run | INFO | It passes locally from a cached binary; the all-features test job is out by decision D10 |

### Stub-panic gate residual limits (assessment)

The fix report (iteration 5) states these limits:
- In five token shapes inside a macro invocation, the gate does not read a stub name as the macro: `$todo`, after `.`, before `.`, before `::`, and before a single `:`. A `macro_rules!` matcher can still rebuild a panicking call from them.
- `include!(concat!(..))` is not resolved.
- A file reached only through `include!` resolves child modules from its own path.
- `closure_end`'s `|` handling is a search result, not a proof.

**Assessment: these do not undermine the goal.**
- Every shape needs deliberate metaprogramming that takes a stub name apart and rebuilds it. A contributor writing an unfinished body writes `todo!()` / `unimplemented!()` directly, and that is caught in every syntactic position I probed (fn body, match arm, after a raw C-string literal).
- The report measured the real tree with no bare `todo`/`unimplemented` identifier outside comments and no `include!`. That matches the gate's 0 findings and my naive grep, where all sampled hits are in cfg(test) code.
- The SC asks for a guard that fails on a planted stub, and it does.
- The limits are documented in the script header with a rustc-checked counterexample, so they read as stated limits, not false assurance.

What remains is a hardening question, not a correctness gap.

### Human Verification Required

None needed for goal achievement. The fix report marks WR-03 "requires human verification (logic changes in a CI gate)". That is a review sign-off on the gate's internals, covered here by the 88-assertion harness, my planted-stub run and the rustc-oracle comparison recorded in 55-REVIEW-FIX.md. These items are CI-proof-of-record and were not run locally:
- the 82% coverage floor (`cargo-llvm-cov` absent)
- `actionlint` on the new `ci.yml` step

Both are milestone-wide gates, not Phase 55 success criteria. They are already tracked (WINDOWS rows 83/84 style).

### Gaps Summary

No blocking gaps. All five roadmap success criteria hold in the code at HEAD `e332ecc3d`, proven by running the code rather than by reading the SUMMARYs:
- the stub gate passes the real tree and fails on a planted stub
- the typed refusals and fences pass their tests
- the unpriced halt is asserted end to end through row, SSE and webhook
- the run-API tests pass in a namespace with no network, and the full lib suites pass with only loopback up
- the cancel test survives repeated runs under load
- single-feature clippy and both rustdoc bars are clean with zero rustdoc allows anywhere

FIXES-04 is legitimately re-homed: the D-05 decision is recorded in every register it names and rests on the measured exhaustiveness of two public enums. FIXES-05 is legitimately closed as verified-not-wrong under the field's documented contract.

Four advisory warnings remain, all documentation accuracy rather than runtime behaviour. They come from un-fixed review Info findings IN-05/06/07/09/11. The runtime status is correct, but two rustdoc sites (one public) and three operator-facing docs describe the unpriced halt inaccurately ("records `Failed`", only two reasons listed, "resumes"). The base-layer review also overstates the file-fetcher helpers as producing no made-up value, while the callers default to `0` / `(0, 0)`. Fix these before the Phase 58 pre-tag audit; they do not block Phase 56.

---

_Verified: 2026-11-02T01:00:58Z_
_Verifier: Claude (gsd-verifier)_

## Post-verification follow-up (orchestrator)

Advisory items 1-3 above were wording fixes and were applied before the phase closed; item 4 stays advisory.

- **1 (unpriced reported as Failed):** `src/application/services/run/worker.rs` `run_agent` rustdoc now says the run records `Halted` with the `model_unpriced` reason. `trace.rs` `RunFinished.halt_reason` is left as is: an unpriced model halts at dispatch, before the engine runs, so no `RunFinished` carries that reason, and the two reasons the rustdoc lists are the only ones the engine emits.
- **2 (promised resume):** `allowance.rs` `HaltReason::ModelUnpriced`, `docs/src/api-reference/platform-api.md` and `MIGRATION.md` M-B-31 now say to resubmit the run, because an agent-kind run writes no Waypoint and so has no checkpoint to resume from.
- **3 (unknown reported as zero):** the three `create_*_content` callers in `file_content_fetcher.rs` now say an unknown figure is recorded as `0` / `(0, 0)` because the content types have no "unknown" representation, and the `55-BASE-LAYER-REVIEW.md` row says the same. No behaviour change.

Re-checked after the follow-up: `cargo fmt --all --check`, `cargo clippy -p paladin-content -p paladin-ai-core -p paladin-ai --lib -- -D warnings`, `cargo doc -p paladin-ai-core --no-deps` (0 warnings), `make check-gates` and `make api-surface` (unchanged).
