# Phase 55: Correctness Fixes, Test Hygiene & Lint Bar - Research

**Researched:** 2026-11-01
**Domain:** Rust workspace hygiene (hexagonal / DDD, 12 published crates): stub-panic guard, typed-error fencing, one halt-reason variant, a hermetic wiring seam, single-feature clippy, the rustdoc bar
**Confidence:** HIGH. Every figure below was measured in this sandbox on 2026-11-01 against `a78338ddb`, except the items tagged `[ASSUMED]`.

## Summary

All the decisions are locked in `55-CONTEXT.md`, so this is a measurement report plus a list of traps. Three results change how the plans should be sized or ordered.

First, the rustdoc bar is bigger than ADR-0033's 309 figure. With the eight crate-level allows removed, the default bar (`cargo doc --workspace --no-deps`) produces **474** diagnostics: `paladin-ports` 187, `paladin-ai` (facade) 140, `paladin-llm` 117, `paladin-storage` 27, `paladin-notifications` 3. They are only two classes, 444 `unresolved link` and 30 `redundant explicit link target`. There are **zero** `invalid_html_tags`. The all-features `-D warnings` bar gives a *different* set, 455 (ports 187, facade 135, llm 105, storage 28, notifications 0), so a fix must be checked against both bars. 380 of the 474 diagnostics carry no source span, so they have to be located by grepping the printed line text.

Second, FIXES-03 does not need TESTS-01 first. `worker_tests.rs` already holds a hermetic regression test for the exact arm, `agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm`. It asserts `Failed` and is the red-then-green flip for D-04. The wiring seam of D-08 is only needed for TESTS-01 itself.

Third, a premise of D-06 (FIXES-05) conflicts with the field's own contract. `ModelProfile.reasoning` is documented as "whether the **default call** to this model **returned reasoning text**". The 50.1 probe row for `claude-opus-4-8` records `reasoning text: absent`. The 41 tokens in the 51 probe come from a non-default request (`max` paired with `thinking: adaptive`) and are token counts, not text. See Open Question 1; it needs a decision before the executor flips the flag.

Also measured: `FIXES-01`'s 26 sites reproduce exactly (6 + 10 + 10 in 3 files), but only if the guard resolves out-of-line `#[cfg(test)] mod x;` files. A naive per-file scan reports 31 (five false positives in `engine/test_support.rs`).

**Primary recommendation:** plan five streams, in this order of dependency. (1) the guard script plus the 20 storage `Err` bodies plus the dead-file delete. (2) `HaltReason::ModelUnpriced` plus the worker arm, flipping the existing worker test, plus docs, OpenAPI and changelog. (3) the `build_run_api` `pub(crate)` seam, the two hermetic tests, the cancel-test inspector and the boot warning. (4) the `paladin-llm` single-feature clippy and TESTS-04/05 fixes, the seed row, the commander pointers. (5) CIREL-08, five crate-by-crate commits, each green under both bars. Then the base-layer review, the register edits and the semver run.

<user_constraints>
## User Constraints (from CONTEXT.md)

The full text of every decision is in `55-CONTEXT.md` and is binding as written there; this section copies the decision headlines verbatim (the bold lead sentence of each) so the planner sees them first. Where this research found a measured conflict it is raised under Open Questions, not decided here.

### Locked Decisions

- **D-00a:** Gates as Phase 54 D-00a: 82 % workspace line coverage (ADR-0006), `make clean-code` (which runs `make doc-check`, both rustdoc bars), `make check-gates`, `make test-shell-guards`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change), `make security`, `make check-migration-allowlist`, `make check-version-strings`, `make check-doc-config` for any fenced YAML touched in `docs/src`, `shellcheck`/`actionlint` for every script and workflow edit, and the manual credential-handling review. No `unwrap`/`expect`/`panic!` in library code. TDD: every behaviour watched failing first.
- **D-00b:** Semver as Phase 53 D-02 / Phase 54 D-00b: every public change is judged at `--release-type patch` against `0.11.0` for all twelve published packages; milestone stays `v0.11.1`; allowlist stays at zero entries and §11.2 header-only. A flagged change is **reshaped to additive or dropped, never registered**. Behavioural changes go in `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`, never a §11.2 row. No `#[deprecated]` anywhere.
- **D-00c:** Frozen records: `MIGRATION.md` §9/§10, the root `CHANGELOG.md` `[0.11.0]` and every crate changelog's published `[0.11.0]` are byte-frozen; corrections go in `[Unreleased]`. Logging stays on the `log` facade. No new ADR expected; ADR-0033 is amended in place with a dated note; if one is needed it takes **0064**.
- **D-00d:** B4 unpriced reachable models get a boot/admission warning, workflow assistants not refused; D10 the eight `#![allow(rustdoc::…)]` removed, all-features CI test job stays out; G3 base-layer items reviewed and fenced, not finished; **stubs that panic are fenced with an `Err`, never finished**.
- **D-00e:** Prove-then-lock: re-measure every enumerated site before changing it; record what is already closed with the proving command; locate by content, never by line number.
- **D-00f:** TESTS-01's hermetic wiring seam lands before FIXES-03 (see Open Question 5: the existing worker harness makes this ordering optional); CIREL-08 is planned with TESTS-03/04.
- **D-01:** **The guard is an offline shell gate in `make check-gates`, zero exemptions** (`scripts/check-no-stub-panics.sh`, bash plus embedded `python3`, no argument, reports `file:line`, "scanned zero files" is a named failure, scans publishable packages' `src/`, comments/strings blanked, `#[cfg(test)]` items exempt, a harness test with fixtures, a Makefile target in `check-gates` and a CI step).
- **D-02:** **Twenty storage bodies return an existing `RepositoryError` variant; the dead file is deleted** (`Err(RepositoryError::QueryError(..))`, one greppable message shape naming method and backend; rustdoc says unfinished and points at `55-BASE-LAYER-REVIEW.md`; a feature-gated table test per backend watched failing first; `content_filtering_service.rs` deleted with its two commented `mod` lines; CHANGELOG and MIGRATION §11.1 entries).
- **D-03:** **The review is done in this phase and recorded; removal is off the table this milestone** (`55-BASE-LAYER-REVIEW.md`, one row per item; `Field`, `Collection`/`CollectionType`/`Item`, `element`: keep with rustdoc; the two versioning services: keep and fence with existing `VersioningError` variants; `planning_service.rs` placeholder: fence; `file_content_fetcher.rs`: fence; the todo closes on the review file).
- **D-04:** **`Unpriced` records `Halted` with a new, additive `HaltReason::ModelUnpriced`** (`{"reason":"model_unpriced","model":"…"}`, `as_str()` `"model_unpriced"`, herald line `⛔ halted: model <name> has no treasurer.pricing row`; worker arm via `persist_agent_halt`; docs tables; `make openapi` if needed; `MIGRATION.md` §11.1 and CHANGELOG).
- **D-05 (FIXES-04): `RunStatusValue::Cancelled` is re-homed to the next minor, with the reason recorded; no code change in `paladin-eval` this phase** (record-keeping only: REQUIREMENTS deferred list, traceability row, ROADMAP SC2 dated note, rustdoc on `RunStatusValue` and `Assertion::RunStatus`, `eval-harness.md`, `deferred-items.md`).
- **D-06 (FIXES-05): the `reasoning` flag follows the two recorded probes, no live re-probe** (`.with_reasoning(true)`, comment cites the evidence, registry test pins it; escape clause: "If the executor's read of the records finds no reasoning tokens anywhere, the flag stays `false` and the evidence file says so").
- **D-07 (FIXES-06): one boot-time `warn!` per unpriced model that a metered principal can reach, in `build_run_api_with_cadence`, beside the TENANT-08 assertion** (pure function, `PRICING_LOG_TARGET`, workflow-assistant graph-node models excluded and said so, boot never refuses, WINDOWS row 67 dispositioned).
- **D-08:** **An additive wiring sibling injects the run engine's `PaladinPort`; production is unchanged** (the two `build_run_api_persists_*` tests inject a test-local scripted port, drop the `OPENAI_API_KEY` dance and `serial`, pass with the network off; WINDOWS row 73 to `fixed`).
- **D-09:** **The cancel test waits on registration, not time** (`LocalRunTokens` read-only inspector, bounded poll inside the 30 s outer timeout, flake proof at least 20 runs under the full parallel load).
- **D-10:** **Lint leftovers are fixed at the source, never silenced** (TESTS-03 feature-scoped helpers, no blanket `#[allow(dead_code)]`; TESTS-04 the four named sites; TESTS-05 one `pub(crate) const SELF_HOSTED_PROVIDERS`; TESTS-06 keep the running commander tests, replace the four `TODO:` blocks with a pointer).
- **D-11:** **All eight allows go in this phase, measured first, fixed at the source, no allow re-added at any level** (crate-by-crate commits, each green under both bars; ADR-0033 amended in place).
- **D-12:** **`docs/src/appendix/doc-coverage-report.md` is regenerated as a dated measurement record, not a per-item coverage table.**
- **D-13:** One `55-CORRECTNESS-EVIDENCE.md` (prove-then-lock results), `55-BASE-LAYER-REVIEW.md` (D-03), `scripts/check-semver.sh` on the phase head with its twelve lines recorded, `deferred-items.md` carrying FIXES-04 and the Deferred Ideas.

### Claude's Discretion

- Exact names: the guard script, the wiring sibling or overrides struct, the inspector, the registry iterator, the boot-warning function, the `HaltReason` variant's field name (D-01, D-04, D-07, D-08, D-09).
- Error and warning wording within the house shapes (names the entry or the model, never a key value; backend text only to `log::error!`).
- The guard's internals: the python/bash split, the comment-blanking and brace-tracking implementation, fixture layout (D-01).
- `Subtask.expected_output`'s honest value (D-03) and whether `ingest_content` refuses or stays a logged no-op, per the caller read the review file records.
- Plan and commit grouping; which crate's rustdoc fixes land in which plan (D-11).
- Whether WINDOWS rows 67 and 73 are closed in this phase's final plan or handed to Phase 57 with evidence pointers (the row text must cite the test names and the warning test).
- Whether the boot warning shares `paladin-llm`'s warn-once set (D-07).

### Deferred Ideas (OUT OF SCOPE)

- WINDOWS rows 85-87 (53-REVIEW WR-01..03); the News API key travelling in the URL (`D-52-03-01`); refusing `${…}` in the treasurer webhook `secret` (Phase 54 WR-04).
- `RunStatusValue::Cancelled` and the scenario schema refresh (next minor, D-05).
- Retiring or partially retiring the base-layer items and the entity-sense identifier renames (next minor); finishing the content / general-data system (a feature milestone).
- Pricing-coverage check at graph registration and enumerating workflow assistants' node models (needs an `AssistantResolver` listing method).
- Sharing the warn-once set between boot and settle time (only if free).
- An `#[ignore]` inventory in the testing guide (E8); the feature-stub TODOs in `trigger.rs`, `service_runner.rs`, `council_service.rs` (E9); the all-features CI test job (D10's second half); refreshing `.planning/codebase/CONCERNS.md` and `TESTING.md`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FIXES-01 | No `todo!()`/`unimplemented!()` reachable outside `#[cfg(test)]` in a published crate; grep guard | 26-site inventory re-measured (Code Inventory 1); working scanner prototype that resolves out-of-line test modules (Code Examples 1); gate template, harness, Makefile and CI wiring (Architecture Patterns) |
| FIXES-02 | Base-layer / content items each carry a recorded keep / fence / remove decision | Surface read of all seven items, `VersioningError` is exhaustive, two fabricated-value sites, `expected_output` is injected into a prompt, `ingest_content` has no shipped caller (Code Inventory 5) |
| FIXES-03 | `Unpriced` on an agent-kind run records `Halted` | Every `HaltReason` touch point, the one worker arm, the existing worker test to flip, docs lines that state `Failed` (Code Inventory 2) |
| FIXES-04 | `RunStatusValue::Cancelled` | Re-homed by D-05; the record-keeping targets are listed (Code Inventory 6) |
| FIXES-05 | `claude-opus-4-8` `reasoning` flag matches the probe record | Quoted probe lines, the field's documented contract, sibling rows with the same evidence, the three test pins (Code Inventory 7, Open Question 1) |
| FIXES-06 | Boot warning once per unpriced reachable model | `AgentRegistry::list()` already exists (no new iterator), `AllowanceConfig::is_empty()` is the "metered principal exists" test, pure-function-plus-logger precedent (Code Inventory 3) |
| TESTS-01 | The two `build_run_api_persists_*` tests hermetic | Reproduced the failure with no proxy; exact seam; double templates (Code Inventory 4) |
| TESTS-02 | `local_cancel_signals_token` waits on registration | `LocalRunTokens` API, the test shape, a `pub(crate)` `#[cfg(test)]` inspector, baseline flake rate (Code Inventory 4) |
| TESTS-03 | Single-feature clippy on `paladin-llm` clean | Full 13-combination matrix with every warning and its fix site (Code Inventory 8) |
| TESTS-04 | Phase 51 lint leftovers cleared | Today's state per named site; `thinking_effort_wire_value` is already gated, the live offender is `chat_effort_wire_value` (Code Inventory 8) |
| TESTS-05 | Self-hosted provider pair defined once | 51 IN-01 reproduces: two consts, three use sites (Code Inventory 8) |
| TESTS-06 | `commander.rs` TODO stubs point at the relocation | Four tests mapped one-to-one to the four relocated tests (Code Inventory 9) |
| CIREL-08 | Eight crate-level rustdoc allows removed; both bars green | Per-crate, per-bar, per-class counts; fix pattern per class; default-vs-all-features divergence (Code Inventory 10) |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Stub-panic guard | Build / CI gate (shell + python) | Makefile `check-gates`, `ci.yml` gate-scripts job | Release gates in this repo are offline shell scripts; the Rust tree-scan test is reserved for vocabulary freezes |
| `ContentListRepository` refusal | Infrastructure adapter (`paladin-storage`) | Port `paladin-ports` (read-only, exhaustive error enum) | The unfinished half lives in two adapters; the port and its error enum cannot change under the patch gate |
| `HaltReason::ModelUnpriced` | Core domain (`paladin-core` `allowance.rs`) | Application worker (`run/worker.rs`), Web DTO docs/OpenAPI | The reason is a domain value; the worker maps an admission outcome to it; the web layer only renders it through `HaltReason::wire_json` |
| Boot warning for unpriced models | Application wiring (`run_api_wiring.rs`) | Core `PriceTable`, web `AgentRegistry` (read-only `list()`) | The one place Treasurer, price table and registry meet; advisory only, never refuses |
| Hermetic `PaladinPort` injection | Application wiring (`pub(crate)` seam) | Test module doubles | Production composition root unchanged; the test supplies the outbound port |
| Cancel-signal inspector | Application service (`run/cancel.rs`) | `#[cfg(test)]` only | Read-only test hook, no public surface |
| Rustdoc links | Each crate's own docs | Both CI bars | Links resolve differently per crate scope and per feature set |

## Standard Stack

No new dependency is introduced by this phase. Everything uses what the workspace already pins.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust toolchain | 1.97.1 stable (`rust-toolchain.toml`); `nightly-2026-09-20` only for `make api-surface` | Build, clippy, rustdoc | Already installed here [VERIFIED: `rustup toolchain list`] |
| `thiserror` / existing `RepositoryError`, `VersioningError` | workspace | Typed errors for the fences | Both enums are exhaustive, so only existing variants are usable [VERIFIED: `repository_port.rs:15`, `entity_version_service.rs:29`] |
| `log` | workspace `0.4` | Boot warning under `PRICING_LOG_TARGET` | House rule: log facade only [CITED: 55-CONTEXT D-00c] |
| `serial_test` | `3.2.0` (dev-dep) | Existing env-var serialisation; the two TESTS-01 tests stop needing it | Already a dev-dependency [VERIFIED: root `Cargo.toml`] |
| `tokio::time` | workspace | Bounded poll in the cancel test | Already used by the test |
| `python3` | 3.13.16 (`tomllib`) | The guard's scanner | Same as `check-crate-names.sh` / `check-version-strings.sh` [VERIFIED: `python3 --version`] |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `cargo-semver-checks` | installed | `scripts/check-semver.sh` | Phase-head proof (D-13); one package took 46 s warm [VERIFIED] |
| `shellcheck` | installed (`/usr/bin/shellcheck`) | `make lint-shell` for the new script and harness | Every script edit |
| `cargo-audit`, `cargo-deny` | installed | `make security` | Phase head |
| `actionlint` | NOT installed | `ci.yml` edit | Record an `unrun-verify` row or wait for CI (precedent: WINDOWS rows 83, 84) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shell+python guard | Rust tree-scan test like `entity_rename_guard.rs` | Rejected by D-01; the python scanner below reuses the same blanking technique |
| `paladin_eval::ScriptedLlm` for the scripted port | Test-local `PaladinPort` double | `paladin-eval` depends on workspace crates, so a root dev-dependency inverts the layering (D-08) |

**Installation:** none. **Package legitimacy:** no external package is added; the Package Legitimacy Audit is not applicable.

## Architecture Patterns

### System Architecture Diagram

```
FIXES-01 guard
  Cargo.toml + crates/*/Cargo.toml --tomllib--> publishable packages (publish != false)
        |                                           (crates/doc-examples is out)
        v
  each pkg/src/**/*.rs --blank comments+strings--> find #[cfg(test)] item spans (brace tracked)
        |                                          resolve `#[cfg(test)] mod x;` -> x.rs / x/mod.rs (transitive)
        v
  todo!( | unimplemented!( outside test spans/files --> file:line report --> exit 1
  (scanned 0 files => named failure)           Makefile check-gates + ci.yml gate-scripts step
                                               + tests/scripts/check-no-stub-panics_test.sh (fixtures)

FIXES-03 / FIXES-06 / TESTS-01 (all in the facade, feature web-server)
  POST /v1/runs --> RunSubmissionService --admit_for_model--> Treasurer (ModelUnpriced => 422)
        |
        v  queued
  RunWorkerPool::run_once --> run_agent --> derive_dispatch_budget
        |   Derived/Unbounded --> paladin_port.execute_scoped --> completed/failed/halted(boundary)
        |   Halt(reason)      --> emit_agent_run_finished(Halted) + persist_agent_halt --> SSE done + webhook
        |   Unpriced(model)   --> [today] Failed + persist_failure   --> [D-04] Halt(ModelUnpriced{model})
        v
  runs.halt_reason TEXT (wire JSON); GET /v1/runs, SSE `done`, webhook `halted` all call HaltReason::wire_json

  build_run_api_with_cadence (boot)
     validate allowance -> stores -> paladin_port_from_settings_with_cadence  <-- TESTS-01 override point
     -> price_table (~974) -> Treasurer -> [FIXES-06 warn here] -> RunWorkerPool -> assert_treasurer_enforced
```

### Recommended Project Structure (additions only)
```
scripts/check-no-stub-panics.sh                      # new gate (name is the planner's)
tests/scripts/check-no-stub-panics_test.sh           # new harness, auto-discovered by make test-shell-guards
tests/scripts/fixtures/check-no-stub-panics/         # optional; the house harness builds trees under mktemp instead
src/infrastructure/web/run_api_wiring.rs             # pub(crate) seam + boot-warning fns + tests
src/application/services/run/cancel.rs               # #[cfg(test)] pub(crate) is_registered
.planning/phases/55-.../55-CORRECTNESS-EVIDENCE.md   # D-13
.planning/phases/55-.../55-BASE-LAYER-REVIEW.md      # D-03
```

### Pattern 1: gate script shape (copy `check-version-strings.sh`)
**What:** bash `set -euo pipefail`, `python3 - "$ROOT" <<'PY'` heredoc, a test-only root override env var (the version-strings harness uses `VERSION_STRINGS_ROOT`), every finding printed before exit, `scanned zero files` is a named failure. The harness `tests/scripts/check-version-strings_test.sh` builds each fixture under one `mktemp -d`, removed by a `trap`, asserts failing cases first, then passing cases, then the real tree, then a checksum that the run mutated nothing. [VERIFIED: read both files]
**Wiring:** add the target to the `check-gates` prerequisite line (`Makefile:268`, currently ten guards); add a step to the CI gate-scripts job after `Check user-facing version strings (FIRST-03)` in `.github/workflows/ci.yml` (CI runs each script itself and does not call `make check-gates`; the job has a `chmod +x scripts/*.sh` step, and the new script file must be committed executable).

### Pattern 2: pure function plus thin logger for boot advisories
**What:** `scan_report(&ScanOutcome) -> Vec<(log::Level, String)>` is pure and table-tested; `report_unattributed_schedules` only logs it (`run_api_wiring.rs:760`). Do the same for FIXES-06: the pure function returns the sorted, deduplicated unpriced set, a second pure fn renders the one line per model, and a thin function logs under `PRICING_LOG_TARGET`. Tests assert on the pure output, so no logger capture is needed.

### Pattern 3: additive-first under the patch gate
Use `pub(crate)` and `#[cfg(test)]` wherever the only caller is in the same crate. A `pub(crate)` item changes neither the semver gate nor `.project/current-exports.txt`.

### Anti-Patterns to Avoid
- **A per-file scanner for the guard.** It reports 31 instead of 26 (the five extra are in `engine/test_support.rs`, reached through `#[cfg(test)] pub(crate) mod test_support;`).
- **Exempting `#[cfg(not(test))]`.** One such item exists (`cadence.rs:338`); it is production code and must not be exempt.
- **`#[allow(dead_code)]` to clear TESTS-03.** D-10 forbids it; gate the helper on the same cfg as its only users.
- **Editing `crates/paladin-storage/migrations/sqlite/013_add_run_halt_reason.sql` to mention the new reason.** `sqlx migrate` checksums applied migrations; leave it (a comment edit changes the checksum). [ASSUMED: sqlx checksum behaviour, standard; the file is also frozen history]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Publishable-package discovery | A hand list or directory-name filter | `tomllib` over root and `crates/*/Cargo.toml`, skip `[package] publish = false` | `crates/doc-examples` is the only exempt one; matches `check-crate-names.sh` |
| Comment/string blanking | A regex that strips `//` | The nested-block-comment and raw-string-aware blanker in Code Examples 1 | `todo!` appears in doc examples, strings and `/* */`; naive stripping misreads `'a` lifetimes and `"//"` |
| Halt wire form | A second JSON builder | `HaltReason::wire_json` | It is the one caller-facing builder (GET, SSE, webhook, agent response) |
| Scripted LLM for the wiring tests | New crate dependency | A ~25-line `PaladinPort` double (`AlwaysSucceedsPaladinPort`, `worker_tests.rs:1103`) | Layering (D-08) |
| Machine-applying rustdoc suggestions | A regex rewrite of 30 redundant links | `cargo doc --message-format=json` suggestions, or hand edit | The 30 redundant-link diagnostics all carry spans and a `help: remove explicit link target` suggestion [ASSUMED: JSON carries the suggestion] |

**Key insight:** every item in this phase is a small edit at a precisely located site; the risk is in the cross-cutting gates (two rustdoc bars, 13 feature combinations, patch-level semver), not in the code.

## Code Inventory

### 1. Stub sites (FIXES-01) [VERIFIED: scanner run 2026-11-01]
Rule: `todo!(`/`unimplemented!(` on a code line (comments and strings blanked), outside any `#[cfg(test)]` item or test-only file, in every `src/` of a package whose `publish` is not `false`. 624 files scanned, 19 resolved as test-only.

| File | Findings | Notes |
|------|----------|-------|
| `crates/paladin-storage/src/sqlite_content_repository.rs` | 10 | `impl ContentListRepository for SqliteStore`, feature `sqlite` |
| `crates/paladin-storage/src/mysql_content_repository.rs` | 10 | `impl ContentListRepository for MySqlContentRepository`, feature `mysql` |
| `crates/paladin-content/src/services/content_filtering_service.rs` | 6 | commented out of `services/mod.rs` (lines 6 and 9), not valid Rust |
| **Total** | **26** | matches CONTEXT D-00e. Feature-gated files are scanned textually, so `--features sqlite,mysql` adds none. |

Naive-scan trap: `crates/paladin-battalion/src/engine/test_support.rs` (five `unimplemented!` at ~630, 681, 765, 1488, 1577) is included by `#[cfg(test)] pub(crate) mod test_support;` in `engine/mod.rs:90`. Other out-of-line test modules: `paladin-llm` `conformance` and `test_support`, `paladin-storage` `cadence/test_logger`, facade `cli/tests`, `assistant/tests`, `run/{tracer_e2e,worker_tests,cancel_tests,stream_tests,http_surface_tests}`, `run/schedule/tests`, `run/webhook/tests`, `treasurer/tests`, `src/lib.rs` `test_support`. One `#[path]` exists (`src/application/cli/commands/eval.rs:34`) and points into `tests/`, so it is outside `src/`. cfg attribute forms in the tree: `#[cfg(test)]` 545, `all(test, feature = "mock")` 4, `all(test, not(...))` 2, `all(test, feature = ..., ...)` 1, `not(test)` 1. [VERIFIED: grep]

After the fixes the expected result is 0; after deleting only the dead file it is 20.

`ContentListRepository` (`crates/paladin-ports/src/output/repository_port.rs:84`) is a synchronous trait with ten methods, all `Result<_, RepositoryError>`: `get_by_id(Uuid)`, `get_by_name(&str)`, `save(&ContentList)`, `update(&ContentList)`, `delete(Uuid)`, `find_all(Option<u32>, Option<u32>)`, `find_by_date_range(DateTime<Utc>, DateTime<Utc>)`, `count() -> u64`, `add_item_to_list(Uuid, &ContentItem)`, `remove_item_from_list(Uuid, Uuid)`. No caller exists outside `paladin-storage` [VERIFIED: grep]. `RepositoryError` variants: `ConnectionError`, `QueryError`, `SerializationError`, `NotFound`, `DuplicateEntry`, `TransactionError`, `MigrationError`, `ContentItemError(#[from])`, `ContentListError(#[from])`; exhaustive, `Debug, Clone, Error`. `QueryError(String)` is the D-02 variant.

Test seam for the table test (in-module, so private fields are reachable): `SqliteStore::new("sqlite::memory:")` is async and works offline; `MySqlContentRepository { pool }` can be built with a lazy pool (`sqlx::mysql::MySqlPoolOptions::new().connect_lazy(..)`) so no server is needed [ASSUMED: `connect_lazy` on `MySqlPoolOptions` in sqlx 0.8; needs a tokio context]. `cargo check -p paladin-storage --features mysql --tests --offline` compiles in 21 s here [VERIFIED].

### 2. `HaltReason::ModelUnpriced` touch points (FIXES-03)
`HaltReason` is `#[non_exhaustive]`, `#[serde(tag = "reason", rename_all = "snake_case")]`; an internally tagged enum accepts a struct variant, so `ModelUnpriced { model: String }` serialises as `{"reason":"model_unpriced","model":"…"}` [VERIFIED: existing newtype variant uses the same tag; serde supports struct variants for internal tagging].

Exhaustive matches inside `crates/paladin-core/src/platform/container/allowance.rs` that need a third arm: `as_str` (~256), `herald_line` (~305), `wire_json` (~345), plus the enum rustdoc and doctests (lines ~210-260, 300, 335-343) and the serde/herald/wire tests at ~1307-1382. No other crate matches the enum exhaustively (it is `#[non_exhaustive]`, so downstream matches already carry wildcards): all other `HaltReason::` references are constructors or tests. Callers that only forward the reason need no edit: `worker.rs:497` (`.map(HaltReason::wire_json)`), `run_controller.rs:473`, `events.rs`, `webhook/mod.rs`, the heralds (`with_halt_reason`).

Worker: `src/application/services/run/worker.rs` has a private `enum DispatchBudget` (~435) with `Halt(HaltReason)`, `Unpriced(String)`, `Derived`, `Unbounded`. The `Unpriced` arm is at ~1691: `emit_agent_run_finished(&dispatcher, RunFinishStatus::Failed, None, 0)` then `persist_failure(.., "model {model} has no treasurer.pricing row")`. `derive_dispatch_budget` (~1873) maps `AdmissionError::ModelUnpriced { model }` to `DispatchBudget::Unpriced(model)`. Smallest change: map it to `DispatchBudget::Halt(HaltReason::ModelUnpriced { model })` and delete the `Unpriced` variant and its arm; the existing `Halt` arm already emits `RunFinishStatus::Halted` with the reason and calls `persist_agent_halt`. Update the rustdoc at `derive_dispatch_budget` ("`ModelUnpriced` records `Failed` naming the model") and the comment above ("the three no-call outcomes").

The existing regression test to flip (not a new one): `worker_tests.rs`, `agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm` (~2976). It uses `harness("gpt-4", "some-other-model", Some(CEILING_NANOS))`, `submit_agent_run`, `h.llm.call_count()`, `drain_events`, `terminal_count`, `only_delivery`. Today it asserts `Failed`, `error` containing `gpt-4` and `treasurer.pricing`, last SSE kind `Error`, delivery `Failed`. Rewrite to the shape of the sibling `agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm` (~2885): status `Halted`, `run.error.is_none()`, `run.halt_reason == Some(HaltReason::ModelUnpriced { model: "gpt-4".into() })`, SSE terminal payload `status: "halted"` and `halt_reason` equal to `wire_json()`, delivery `RunEventKind::Halted`. Rename the test.

Docs and published text that currently state `Failed` or need the third reason:
- `docs/src/api-reference/platform-api.md` ~156-165 (Halted runs: add the third object), ~226-229 (SSE `done` payload list), ~797-798 (**states "records `failed` with an error naming the model"; must change**).
- `docs/src/architecture/treasurer.md` ~34 and ~41 (prose lists `ledger_unavailable` only; no table exists; add the reason and the boot warning).
- `docs/src/getting-started/configuration.md` ~478-480 and ~592 (unpriced statements; add the boot warning there).
- `crates/paladin-web/src/run_controller.rs:445` field doc `RunResponse.halt_reason` names two reasons; editing it changes exactly one description string in `crates/paladin-web/openapi.json` (line ~3984; the OpenAPI schema is `Option<Object>`, so there is no enum). Run `make openapi` (`UPDATE_OPENAPI=1 cargo test -p paladin-web --lib openapi_matches_committed_baseline`). The frozen v0.9 golden gate (`openapi_golden_v0_9.rs`) concerns the three agent routes' `ExecuteResponse.halt_reason`, which carries only `allowance_exhausted`; it is unaffected.
- `crates/paladin-storage/src/run/sqlite.rs` (~326) already reads an undecodable `halt_reason` as `None` and logs the run id, so a rollback to a build without the variant degrades gracefully (CHANGELOG `[0.11.0]` of `paladin-storage` records this). Do not edit `013_add_run_halt_reason.sql`.
- `MIGRATION.md` §11.1: next row id is **M-B-30** (last is M-B-29). CHANGELOG: root `[Unreleased]` and `paladin-core` (has an `[Unreleased]` anchor).

### 3. Boot warning (FIXES-06)
- `AllowanceConfig::is_empty()` (`src/config/treasurer.rs:526`) is `tenants.is_empty() && api_keys.is_empty()`. The Treasurer is built only when it is false (`run_api_wiring.rs` ~957). So "a metered principal exists" is exactly "the Treasurer is built"; the warning belongs inside that `else` branch, right after `treasurer_config.price_table()` (~974) and before `Treasurer::new(..).with_pricing(Arc::new(price_table))` moves it.
- Reachable models: `paladin_web::AgentRegistry::list() -> Vec<(String, Arc<Paladin>)>` **already exists** (`crates/paladin-web/src/agent_registry.rs:190`); each `Paladin` carries `data.model`. No additive iterator is needed, so `paladin-web` need not change. `paladin-server` builds the registry from `settings.agents` (`AgentDefinition.model`, a required `String`) and passes the same `Arc<AgentRegistry>` to `build_run_api_with_cadence` [VERIFIED: `src/bin/paladin-server.rs:127-138`]. `settings.agents` is also a public field if the planner wants a redundant config pass; the registry is the superset.
- Price lookup is `PriceTable::row(&str) -> Option<&PriceRow>` (`cost.rs`, exact model-name match), the same call `derive_from` uses (`treasurer/derive.rs:123`), so the warning and the dispatch refusal cannot disagree.
- Log: `paladin_llm::pricing::PRICING_LOG_TARGET = "paladin::pricing"` (public const). The settle-time warn-once `UnpricedModelWarnings` is private and process-wide; leave it untouched (a boot line plus a first-settle line is acceptable, per D-07).
- Not enumerated, and the warning text plus `treasurer.md` should say so: workflow-assistant graph-node models (`AssistantResolver::resolve` is id-keyed). Two further gaps the planner should name in the docs: **stored agent-kind assistants** (`AssistantRepositoryPort` has `list`, so they are enumerable at boot but are created at runtime afterwards) and **runtime-provisioned agents** (`FacadeProvisioner`). See Open Question 3.
- Existing test file section to extend: `run_api_wiring.rs` tests around `assert_treasurer_enforced` (~1696-1790) and the schedule-scan pure-function tests (~1830-2120).

### 4. Hermetic run API and the cancel test (TESTS-01, TESTS-02)
Reproduction (row 73): `env -u HTTPS_PROXY -u https_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` fails both tests in 22.8 s with `run_api_wiring.rs:3036: the run never reached a terminal status` [VERIFIED]. `api.openai.com` is unreachable through the proxy here (HTTP 000) [VERIFIED]. The module is gated by feature `web-server` (`infrastructure/web/mod.rs:9`), so a plain `cargo test -p paladin-ai --lib` selects 0 tests; `cargo test --workspace` selects them through feature unification.

Seam: in `build_run_api_with_cadence`, the only call that reaches the network is `paladin_port_from_settings_with_cadence(settings, treasury_ledger.clone(), cadence)` (~890). The port is then cloned into `engine_factory`, `base_engine` and `RunWorkerPool::with_paladin_port`. Recommended shape: move the current body into `pub(crate) async fn build_run_api_with_port(.., paladin_port_override: Option<Arc<dyn PaladinPort>>)` that does `let paladin_port = match override { Some(p) => p, None => paladin_port_from_settings_with_cadence(..)? }`, and make `build_run_api_with_cadence` a one-line delegation passing `None`. Signature and behaviour of `build_run_api` and `build_run_api_with_cadence` are unchanged and the public surface does not move (no `make api-surface-update`). This departs from D-08's wording "bodies byte-for-byte" (the body becomes a delegation); see Open Question 2. A public sibling is also semver-additive but forces an API-snapshot refresh and a CHANGELOG row.

Test doubles to copy: `AlwaysSucceedsPaladinPort` (`worker_tests.rs:1103`, returns `PaladinResult { output, usage: TokenUsage::new(11, 7), ..Default::default() }`), `UnusedPaladinPort` (`cancel_tests.rs:50`, `run_api_wiring.rs` ~1727), `ScopeRecordingPort` (`worker_tests.rs:2272`). `PaladinPort` needs `execute`, `execute_stream`, `validate`. A canned success is enough for the two tests (they need a terminal status and `run_traces` rows). The two tests currently use `persisted_trace_records` (~2950), which registers a `FailingExecutor` in the `AgentRegistry` that the agent-kind path never consults, sets `OPENAI_API_KEY`, and carries `#[serial_test::serial(paladin_run_api_wiring_openai_api_key)]`. With the override the key is not needed and `serial` can go from these two. The other tests in the module (16 `set_var("OPENAI_API_KEY")` sites in the module, one of them in the shared `persisted_trace_records` helper) still set the key and keep the serial group (they build the real default provider but never dispatch, except `wired_treasurer_records_a_warn_notice_for_the_admitted_run` and `build_run_api_wires_the_allowance_warn_path`, which admit a run a live worker may pick up; both assert only notices, so they pass offline but still attempt egress; leave them).

`LocalRunTokens` (`cancel.rs:179`, `Arc<RwLock<HashMap<RunId, CancellationToken>>>`): `new`, `register`, `remove`, `cancel_if_local`. Add `#[cfg(test)] pub(crate) async fn is_registered(&self, run_id: &RunId) -> bool { self.tokens.read().await.contains_key(run_id) }` (a plain `pub(crate)` would be `dead_code` outside tests). `pool.local_tokens()` already returns a clone that shares the map. In `local_cancel_signals_token` (`cancel_tests.rs` ~312-375) replace the `tokio::time::sleep(Duration::from_millis(50))` and its comment with a poll such as `while !tokens.is_registered(&run_id).await { tokio::time::sleep(Duration::from_millis(5)).await }`, bounded by the existing 30 s outer `timeout`. Baseline measured: 0 failures in 60 isolated runs of the test binary under 8 CPU-burning loops on 4 cores [VERIFIED]; the flake only appears under the full parallel suite, so D-09's 20-run proof must be the full-suite run. The facade `--lib` binary (1,474 tests) takes about 43 s to run once, so 20 runs is about 15 minutes after one build.

### 5. Base-layer and content items (FIXES-02)
Facts for `55-BASE-LAYER-REVIEW.md` [VERIFIED: files read]:

| Item | Size and shape | Finding |
|------|----------------|---------|
| `Field<T>` (`base/entity/field.rs`) | 29 lines, `fid`, `nid`, `value`; the explanation is a `/* */` block comment, not rustdoc | `Field::new(value)` invents a random `nid` (parent id) that points at nothing; document it; `nid` is the retained entity-sense identifier |
| `CollectionType<T>`, `Item<T>` (`entity/collection.rs`) | 77 lines, working container, serde on `Item` | Works; `/* */` header only; keep, convert to `///` |
| `element` (`component/element.rs`) | 25 lines, comment-only module | Convert the block comment to `//!` rustdoc; nothing to fence |
| `FieldVersionService<T>` (`service/field_version_service.rs`) | 600 lines, in-memory repository and tests | Fabricated value: `get_field_version_history` falls back to `Uuid::new_v4()` for `node_id` when the field has no versions (~487) |
| `CollectionVersioningService<T>` (`service/collection_versioning_service.rs`) | 1,115 lines | Fabricated values: `detect_collection_changes` emits `ItemAdded(Uuid::new_v4())` / `ItemRemoved(Uuid::new_v4())` marked `// Simplified` (~415-417), surfaced through the public `CollectionVersionComparison.detected_changes` |
| `VersioningError` (`service/entity_version_service.rs:30`) | `VersionNotFound(String)`, `EntityNotFound(Uuid)`, `InvalidVersionNumber(u32)`, `VersioningDisabled(Uuid)`, `StorageError(String)`, `SerializationError(String)` | **Exhaustive** (no `#[non_exhaustive]`): only these variants are usable |

The only consumers outside `base/` are `base/mod.rs` and `tests/entity_rename_guard.rs` (a vocabulary freeze that blanks comments and strings; documentation edits are safe, identifier renames are not). `.project/Milestone_1-MVP` holds ten Epics (Paladin, Garrison, Arsenal, Commander, Armory CLI and so on) and **no design record** for the content / general-data system; a grep for "general data", "data orchestration" over `docs/Design`, `notes`, `.project`, `.planning/decisions`, `docs/src` finds only the inventory, the scoping note, ADR-0060 and the todo. The review file should say "no design record exists beyond the todo's own account" rather than invent one.

`planning_service.rs`: line ~417 builds `Subtask::new(id, description, "Expected output from subtask execution".to_string())`. `Subtask::new(String, String, String)` and `expected_output: String` (`planning.rs:127,157`). The field is **read** in `execute_subtask` (~512): the prompt contains `EXPECTED OUTPUT: {}` and "Be concise and focused on the expected output". So setting an empty string alone would produce a dangling "EXPECTED OUTPUT: " line in every subtask prompt; the fence must also adjust that prompt (omit the line when empty, or use the subtask description as the honest value). A core doctest at `planning.rs:266` asserts `"Analysis report"` and is unaffected.

`file_content_fetcher.rs`: three metadata helpers return `None` (already honest, TODOs inside); `impl ContentIngestionPort for FileContentFetcher` has `fetch_content` (works) and `ingest_content`, which validates a URL then falls through to a `// TODO: Implement actual ingestion logic` and returns `Ok(())` (~152). Callers: none outside the file's own test `test_ingest_content_validation` (~308), which asserts `Ok` for valid content; `tests/content_ingestion_pipeline.rs` imports the trait but calls only `fetch_content`; `docs/src/user-guides/content-processing.md:67` documents `fetch_content` only and calls the adapter internal `#[doc(hidden)]`. So no shipped path depends on the no-op `Ok(())`, and D-03's refusal is safe; the unit test must change from `is_ok()` to `is_err()` for the valid case (watch it fail first).

### 6. FIXES-04 record-keeping (D-05)
Edits only: `.planning/REQUIREMENTS.md` (move FIXES-04 to the v2 list beside PROV-05 and CIREL-10, traceability row line ~342 to re-homed), ROADMAP Phase 55 SC2 dated reconciliation note (the Phase 54 FIRST-08 pattern, ROADMAP ~429), rustdoc on `RunStatusValue` (`crates/paladin-eval/src/scenario.rs` ~569) and `Assertion::RunStatus`, `docs/src/user-guides/eval-harness.md` (the `run_status` row at line 77), the phase `deferred-items.md`, and inventory rows B5 / 42-06 closed by pointer. Rustdoc-only edits in `paladin-eval` do not touch the semver gate. `docs/src/user-guides/eval-harness.md` contains YAML, so run `make check-doc-config` after editing.

### 7. FIXES-05 evidence (quoted)
- `crates/paladin-llm/src/registry/seeded.rs`, `fn claude_opus_4_8` (~188), `.with_reasoning(false)` under the comment "Probe 1 (the default call) carried no reasoning text; the listing capabilities were truncated in the transcript, so reasoning is not evidenced."
- `50.1-SEED-PROBE.md`, `### Probes: claude-opus-4-8` row 1: usage fields include `output_tokens_details.thinking_tokens` (also on rows 3, 9, 10, 11); the last column, `reasoning text`, is **`absent`**. Listing row 852: `effort.supported: true`, capabilities truncated at 808 bytes. A usage-field *name* being present does not show the model reasoned.
- `51-EFFORT-PROBE.md`, `### claude-opus-4-8`: `output_config` rows `low=0 medium=0 high=0 xhigh=0 max=0` reasoning tokens; `output_config+adaptive` `low=0 … xhigh=0 max=41`; `thinking_object` off `rt=0`, auto `rt=0`. 41 tokens occur only for `max` paired with `thinking: adaptive`.
- The field contract (`crates/paladin-ports/src/output/model_profile.rs:963`): "Whether the default call to this model **returned reasoning text**. It is a measurement about the model's output, not a statement of what effort the model accepts ... It still drives the reasoning-model timeout default."
- Sibling rows with identical evidence and wording (`claude-haiku-5-5` ~166, the row at ~248) are `false`; rows whose probe 1 "returned reasoning text" are `true` (`claude-fable-5` ~285, ~360).
- Pins to update together if the flag changes: `seeded.rs`, `registry/mod.rs:1702` (comment and `assert!(!p.reasoning)` in a neighbouring test) and `registry/mod.rs:1734` (`anthropic_opus_4_8_seed_matches_the_transcript`, `assert!(!p.reasoning)`), and the `paladin-llm` CHANGELOG (no `[Unreleased]` anchor yet).
- Effect of `true`: `config/layers.rs` ~441-453 applies the 300 s reasoning-timeout default when the resolved model `is_reasoning`.

### 8. `paladin-llm` single-feature clippy (TESTS-03/04/05) [VERIFIED: 13 runs, `--offline`, exit 0 each]
Command: `cargo clippy -p paladin-llm --tests --no-default-features --features <f>`; provider features in `crates/paladin-llm/Cargo.toml`: `openai`, `anthropic`, `deepseek`, `kimi`, `qwen`, `grok`, `ollama`, `openai-compatible`, `gemini`, plus `mock`, `vision` (= openai + base64), `openai-embeddings` (= openai); default is `["openai","mock"]`. All warnings are in the `lib test` target.

| Combination | Warnings | Combination | Warnings |
|-------------|----------|-------------|----------|
| default | 1 | gemini | 7 |
| openai | 5 | ollama | 5 |
| anthropic | 7 | openai-compatible | 5 |
| deepseek | 4 | mock (no provider) | 10 |
| kimi | 6 | vision | 5 |
| qwen | 8 | openai-embeddings | 5 |
| grok | 5 | | |

Warning classes and their fix site (locate by name; line numbers drift):
1. `unused import ReasoningFallback` at `provider_factory.rs` ~1935 in `mod reasoning_defaults`: every combination except `deepseek` **including the default build** (so `cargo clippy --tests -p paladin-llm` with default features is also not clean today; CI's all-features run hides it).
2. `unused variable chat` at `provider_factory.rs` ~1240 in `wire_cases()`: `anthropic`, `gemini`, `mock`, where no chat-completions provider is compiled. Scope the closure with the `#[cfg(any(feature = "openai", "deepseek", ...))]` set its `cases.push` users carry.
3. `PinnedSpread` and `pin_waiter_spread` (`cadence.rs` ~359, ~372, both `#[cfg(test)]`) and `fallback::test_support::ProfiledStubPort` (`new`, `asked`, `fallback.rs` ~1003-1011, `#[cfg(test)]`): every combination without `mock`. Their only users are tests already gated `#[cfg(all(test, feature = "mock"))]` (cadence tests at `cadence.rs:479`, `fallback.rs:1080`, `pricing.rs` tests). Gating the helpers on `all(test, feature = "mock")` clears them with no allow. The `pin_waiter_spread` import in `fallback.rs` ~1686 sits in a `mock`-gated test module.
4. `chat_effort_wire_value` (`conformance.rs` ~275, `pub fn`): `anthropic`, `kimi`, `qwen`, `gemini`. **`thinking_effort_wire_value` is already `#[cfg(any(feature = "kimi", feature = "ollama", feature = "openai-compatible"))]`** and does not warn in any combination, so the named item of TESTS-04 is already closed (record the proving command); the live offender is `chat_effort_wire_value`, to be gated on the providers whose fixtures call it (openai, deepseek, grok per its doc).
5. `level_spelling` and `levels` (`provider_factory.rs` ~1958, ~1962): `qwen` and `mock`.
6. `mock` alone also flags `UNPROFILED`, `assert_default_applies`, `send`, `stamped`, `Case` and the `probe` macro (the whole `reasoning_defaults` module needs a provider). Decide whether `mock`-only belongs in the matrix; the roadmap says "single-feature", and `mock` has no provider, so gating the module on "at least one provider feature" is the cleaner fix.
7. `engine/cache_key.rs` `NODE_CACHE_KEY_VERSION` link and the `reasoning.rs` redundant explicit links (lines 6 and 9 of its module docs): these are rustdoc, part of the diagnostics counted in 10. `reasoning.rs` also defines `SELF_HOSTED_PROVIDERS` (~257).

TESTS-05: `SELF_HOSTED_PROVIDERS` (`reasoning.rs:257`, private `const`, used at ~317) and `REASONING_WIRE_PROVIDERS` (`config/layers.rs:342`, used at ~499 and in a test at ~671) both spell `["ollama", "openai-compatible"]`. Make the reasoning const `pub(crate)`, import it in `layers.rs`, delete the second const, add a test that both call sites use the same slice. `pub mod reasoning;` is a public module but a `pub(crate)` const does not enter the API snapshot. 51 IN-01 reproduces.

### 9. Commander stubs (TESTS-06)
The four tests are in `crates/paladin-battalion/src/commander.rs` (inline `#[cfg(test)]`), none `#[ignore]`d; each asserts `result.is_ok()` and ends with a `// TODO:` block (~3834, ~3870, ~3917, ~3957). One-to-one relocation in `tests/integration/commander_error_paths_test.rs` (declared in `tests/integration/mod.rs:47`):

| Stub | Relocated test |
|------|----------------|
| `test_error_handling_fail_fast` | `test_fail_fast_stops_on_first_error` |
| `test_error_handling_continue_on_error` | `test_continue_on_error_collects_all_errors` |
| `test_error_handling_retry_then_continue` | `test_retry_then_continue_retries_failed_paladins` |
| `test_partial_failure_handling` (Phalanx) | `test_partial_results_returned_with_errors` (the relocated test builds a Formation; say so in the pointer rather than claiming Phalanx parity) |

### 10. Rustdoc bar (CIREL-08) [VERIFIED: allows removed in a scratch worktree at HEAD; the worktree has been removed]
Eight lines removed: `paladin-ports` lib.rs 51-52, `paladin-llm` 49, `paladin-storage` 20, `paladin-notifications` 14, facade `src/lib.rs` 117-119 (the `#![warn(rustdoc::broken_intra_doc_links)]` above the ports allows stays).

| Crate | Default bar `cargo doc --workspace --no-deps` | All-features bar `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --keep-going` |
|-------|----------------------------------------------|-----------------------------------------------------------|
| `paladin-ports` | 187 (170 unresolved, 17 redundant) | 187 (170, 17) |
| `paladin-ai` (facade) | 140 (127, 13) | 135 (122, 13) |
| `paladin-llm` | 117 (117, 0) | 105 (105, 0) |
| `paladin-storage` | 27 | 28 |
| `paladin-notifications` | 3 | 0 |
| `paladin-ai-core`, `-battalion`, `-content`, `-herald`, `-memory`, `-eval`, `-web`, `doc-examples` | 0 | 0 |
| **Total** | **474** | **455** |

ADR-0033's 2026-09-17 figure was 309 (ports 119, facade 108, llm 69, storage 10, notifications 3), so the surface has grown by 165 since. `invalid_html_tags` hides **0** diagnostics, so that allow was already dead. Timings: the default bar 3 min 55 s (workspace crates compiled fresh, dependencies warm), the all-features bar 1 min 52 s. Only two classes exist.

Divergence between the bars (these links must be code spans, not links, to pass both): default-only (feature-gated targets): `email_notification_adapter`, `push_notification_adapter`, `system_notification_adapter`, `kimi::*`, `qwen::*`, `grok::*`, `ollama::*`, `openai_compatible::*`, `gemini::*`, `openai::OpenAIEmbeddingAdapter`, `crate::openai::vision`, `mysql_content_repository`, `minio`, `cadence::redis`, `scheduler`, `crate::cadence::redis::RedisCadence`, `crate::scheduler::TokioCronSchedulerAdapter`, `cli`, `paladin_content`, `paladin_notifications`, `paladin_storage::treasury::postgres::PostgresTreasuryLedger`. All-features-only (inside `scheduler`-gated modules): `SchedulerPort`, `JobInfo`, `JobRegistration`, `JobSpec::metadata`, `SchedulerPort::get_job_status`, `SchedulerPort::get_job_info`, `TokioCronSchedulerAdapter::record_job_run`, `super::engine`, `adapter`.

Fix pattern per class (D-11):
- **Unresolved bare type from another crate** (the bulk, e.g. `LlmPort`, `ModelProfile`, `TokenUsage`, `ThreadId`, `Namespace`, `TraceSink`, `SanctumPort`): use an explicit target, ``[`ModelProfile`](paladin_ports::output::model_profile::ModelProfile)``, when the crate depends on the target's crate (ports to core, llm to core and ports, facade to all), or bring the item into scope with a `use`. The explicit form is valid exactly because the bare label does not resolve; the redundant-link lint (class below) fires only when the label *does* resolve to the same destination, so check each rewrite against the lint in the same run.
- **Target in a higher layer** (ports docs naming `paladin-web`, `RunSubmissionService`, `crate::application::ports`): plain code span; a leaf crate cannot link upward.
- **Feature-gated target**: plain code span (the only form both bars accept).
- **Hyphenated names** (``[`paladin-core`]`` at `paladin-ports/src/lib.rs:7`): code span without brackets.
- **Markdown headings that look like references** (`## [base]`, `## [cli] (feature `cli`)`, `## [errors]`, `## [platform]`, `## [services]` in `src/core/mod.rs:17ff`): escape the brackets or use code spans.
- **Redundant explicit link** (30, all with spans, e.g. ``[`ArsenalError`](paladin_core::platform::container::arsenal::ArsenalError)`` in `paladin-ports/src/output/arsenal_port.rs:65`): drop the target to ``[`ArsenalError`]`` only where the label resolves in scope (the lint's own help text says so); where it does not resolve, make the label the path.
- **No span** (380 of 474): the diagnostic prints "the link appears in this line:" followed by the source line; locate with `rg -F '<that line text>'` (for example the first one is `crates/paladin-ports/src/input/allowance_admission_port.rs:3`).
- The facade's 140 do not overlap the leaf crates' lines (checked by line text: 0 shared), so they are independent work.

Crate-by-crate order that keeps every head green under both bars: ports (largest), llm, storage, notifications, facade, each commit removing its own allow lines only after its diagnostics are fixed. `cargo doc -p <crate> --no-deps` per crate is a fast inner loop; the facade also needs the whole workspace.

`doc-coverage-report.md` is 48 lines with an "Archived — historical document" banner; `SUMMARY.md:115` links it. ADR-0033's amendment table has the storage allow at line 19 (now 20).

WINDOWS row 56 (`cost.rs` unresolved links, status `open`): `paladin-ai-core` emits zero diagnostics under both bars today, so the row is already fixed in fact; close it with this measurement as evidence when CIREL-08 lands (not named in CONTEXT).

## Common Pitfalls

### Pitfall 1: the guard misreads test-only files
**What goes wrong:** a per-file scan flags `unimplemented!` in `engine/test_support.rs` and similar; or an over-broad rule exempts `#[cfg(not(test))]`.
**How to avoid:** resolve `#[cfg(test)] mod x;` to `x.rs` or `x/mod.rs` (directory is the module file's own directory for `lib.rs`, `main.rs`, `mod.rs`, else the file stem) transitively, and treat only `test` and `all(test, ...)` as exempt. The harness must include an out-of-line test module fixture and a `cfg(not(test))` fixture.
**Warning signs:** the real-tree run says 31 not 26; or the guard passes while a `cfg(not(test))` `todo!` exists.

### Pitfall 2: both rustdoc bars must hold after every crate commit
**What goes wrong:** a link fixed for the default build breaks the all-features build (or the reverse); a link to a feature-gated item passes one bar only.
**How to avoid:** run both bar commands after each crate; make feature-gated targets code spans.

### Pitfall 3: the Unpriced test flips, the SSE and webhook assertions follow
**What goes wrong:** only the status is changed; the terminal SSE kind (`Error` to `Done` with `status: "halted"`) and the delivery event (`Failed` to `Halted`) keep their old assertions.
**How to avoid:** rewrite the whole test to the sibling halt tests' assertions (Code Inventory 2).

### Pitfall 4: `expected_output` is a prompt input
Replacing the fabricated sentence with `String::new()` leaves "EXPECTED OUTPUT: " in every subtask prompt. Fence the prompt too, and pin both in tests.

### Pitfall 5: crate changelog anchors
`paladin-storage`, `paladin-llm`, `paladin-ports`, `paladin-battalion`, `paladin-eval`, `paladin-herald`, `paladin-notifications` have **no** `## [Unreleased]` heading; only `paladin-content`, `paladin-ai-core`, `paladin-memory`, `paladin-web` and the root do [VERIFIED: grep]. CONTEXT assumed anchors for storage and llm. Add the heading above `## [0.11.0]` (never edit `[0.11.0]`); `scripts/finalize-crate-changelogs.sh` requires the anchor at release (Phase 58).

### Pitfall 6: stale `target/` in this sandbox
The prebuilt `target/` embeds a stale `CARGO_MANIFEST_DIR`: three path-reading tests (`exactly_two_schemars_versions_remain`, `no_auto_write_middleware_exists`, `the_repair_loop_is_the_shared_driver`) fail with `No such file or directory` until the crate is rebuilt, and pass after `touch src/lib.rs` [VERIFIED: 3 of 3 passed after the rebuild]. Do not mistake them for phase regressions.

### Pitfall 7: `--lib` on the facade selects no run-API tests
Use `--features web-server` (or `--workspace`), or the evidence will show "0 tests run".

### Pitfall 8: `make api-surface` and `make openapi` produce no diff for `pub(crate)` work
That is correct. A diff means a public item slipped in. The additive public items are only `HaltReason::ModelUnpriced` (core, not in the facade snapshot) and the RunResponse doc string (OpenAPI).

### Pitfall 9: actionlint is absent
The `ci.yml` step cannot be linted locally; record an `unrun-verify` ledger row as rows 83 and 84 did, or rely on the CI Workflow Lint job.

## Code Examples

### 1. Guard scanner (verified against the real tree and planted fixtures)
Embedded as the python heredoc of `check-no-stub-panics.sh`. Real tree: `scanned 624 files (19 test-only), 26 finding(s)` over exactly the three files above. A planted tree proved: a production `todo!`, a `cfg(not(test))` `todo!` and a file reached by `pub mod sub;` all fail; `///` examples, string literals, raw strings, `/* */`, `#[cfg(test)] mod tests {}`, `#[cfg(all(test, feature = "mock"))] mod x {}`, and a transitive out-of-line `#[cfg(test)] mod ext;` containing `mod deeper;` all pass.

```python
# Source: prototype validated 2026-11-01 (same blanking technique as tests/entity_rename_guard.rs)
# Embed in scripts/check-no-stub-panics.sh as the python3 heredoc; the bash wrapper passes the workspace root.
#!/usr/bin/env python3
"""Prototype of the D-01 scanner (check-no-stub-panics): todo!/unimplemented! in publishable src/."""
import glob, os, re, sys, tomllib

STUB = re.compile(r'\b(?:todo|unimplemented)\s*!\s*[\(\[{]')
CFG_TEST_ATTR = re.compile(r'#\s*\[\s*cfg\s*\(\s*(?:test|all\s*\(\s*test\s*,[^\]]*\))\s*\)\s*\]')
INNER_CFG_TEST = re.compile(r'#!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')
MOD_DECL = re.compile(r'\bmod\s+(\w+)\s*;')


def blank(src):
    """Replace comments and string/char literals with spaces (newlines kept), like
    tests/entity_rename_guard.rs. Handles nested block comments and raw strings."""
    out, n, i = list(src), len(src), 0
    def wipe(a, b):
        for k in range(a, b):
            if out[k] != '\n':
                out[k] = ' '
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i); j = n if j < 0 else j
            wipe(i, j); i = j
        elif src.startswith('/*', i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith('/*', j): depth += 1; j += 2
                elif src.startswith('*/', j): depth -= 1; j += 2
                else: j += 1
            wipe(i, j); i = j
        elif c in 'rb' and re.match(r'(?:br|r)#*"', src[i:i + 12]) and (i == 0 or not (src[i-1].isalnum() or src[i-1] == '_')):
            m = re.match(r'(?:br|r)(#*)"', src[i:])
            end = src.find('"' + m.group(1), i + len(m.group(0)))
            j = n if end < 0 else end + 1 + len(m.group(1))
            wipe(i, j); i = j
        elif c == '"' or (c == 'b' and src.startswith('b"', i)):
            j = i + (2 if c == 'b' else 1)
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            wipe(i, j + 1); i = j + 1
        elif c == "'":
            m = re.match(r"'(?:\\.[^']*|[^\\'])'", src[i:i + 12])  # char literal, not a lifetime
            if m: wipe(i, i + len(m.group(0))); i += len(m.group(0))
            else: i += 1
        else:
            i += 1
    return ''.join(out)


def item_end(b, j):
    """Index of the `;` or matching `}` that ends the item whose header starts at j."""
    while True:                       # skip further attributes
        m = re.match(r'\s*#\s*\[', b[j:])
        if not m: break
        depth, k = 0, j + m.end() - 1
        while k < len(b):
            depth += (b[k] == '[') - (b[k] == ']')
            if depth == 0: break
            k += 1
        j = k + 1
    k = j
    while k < len(b) and b[k] not in '{;': k += 1
    if k >= len(b) or b[k] == ';': return k
    depth = 0
    while k < len(b):
        depth += (b[k] == '{') - (b[k] == '}')
        if depth == 0: break
        k += 1
    return k


def analyse(path):
    b = blank(open(path, encoding='utf-8').read())
    if INNER_CFG_TEST.search(b):
        return None, []                                   # whole file is test-only
    spans, test_mods = [], []
    for m in CFG_TEST_ATTR.finditer(b):
        end = item_end(b, m.end())
        spans.append((m.start(), end))
        d = MOD_DECL.search(b, m.end(), end + 1)
        if d and b[m.end():end + 1].lstrip().split('mod')[0].strip() in ('', 'pub', 'pub(crate)', 'pub(super)'):
            test_mods.append(d.group(1))
    hits = [b.count('\n', 0, m.start()) + 1 for m in STUB.finditer(b)
            if not any(a <= m.start() <= z for a, z in spans)]
    return hits, test_mods


def module_dir(path):
    base = os.path.basename(path)
    return os.path.dirname(path) if base in ('lib.rs', 'main.rs', 'mod.rs') else path[:-3]


def main(root):
    packages = []
    for mp in [os.path.join(root, 'Cargo.toml')] + sorted(glob.glob(os.path.join(root, 'crates', '*', 'Cargo.toml'))):
        pkg = tomllib.load(open(mp, 'rb')).get('package')
        if pkg and pkg.get('publish', True) is not False:
            packages.append(os.path.dirname(mp))
    files = sorted(os.path.join(dp, f) for p in packages for dp, _, fs in os.walk(os.path.join(p, 'src')) for f in fs if f.endswith('.rs'))
    if not files:
        print('FAIL scanned zero files'); return 1
    parsed = {f: analyse(f) for f in files}
    test_only = {f for f, (h, _) in parsed.items() if h is None}
    work = list(test_only) + [f for f, (h, t) in parsed.items() if t]
    queue = [f for f in files if parsed[f][1]]
    while queue:                                         # resolve `#[cfg(test)] mod x;` transitively
        f = queue.pop()
        for name in parsed[f][1]:
            for cand in (os.path.join(module_dir(f), name + '.rs'), os.path.join(module_dir(f), name, 'mod.rs')):
                if cand in parsed and cand not in test_only:
                    test_only.add(cand); queue.append(cand)
    # a test-only file's own `mod x;` children are test-only too
    changed = True
    while changed:
        changed = False
        for f in list(test_only):
            b = blank(open(f, encoding='utf-8').read())
            for name in MOD_DECL.findall(b):
                for cand in (os.path.join(module_dir(f), name + '.rs'), os.path.join(module_dir(f), name, 'mod.rs')):
                    if cand in parsed and cand not in test_only:
                        test_only.add(cand); changed = True
    findings = [(f, l) for f in files if f not in test_only for l in parsed[f][0]]
    for f, l in findings: print(f'{os.path.relpath(f, root)}:{l}')
    print(f'scanned {len(files)} files ({len(test_only)} test-only), {len(findings)} finding(s)')
    return 1 if findings else 0

if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))
```

### 2. `HaltReason` variant
```rust
// Source: crates/paladin-core/src/platform/container/allowance.rs (additive on a #[non_exhaustive] enum)
/// The run's model has no `treasurer.pricing` row at dispatch ... (names the model only).
ModelUnpriced {
    /// The model name; never a tenant or key.
    model: String,
},
// as_str:    HaltReason::ModelUnpriced { .. } => "model_unpriced",
// herald_line: format!("\u{26D4} halted: model {model} has no treasurer.pricing row")
// wire_json: serde_json::json!({ "reason": self.as_str(), "model": model })
```

### 3. Boot warning (pure function plus thin logger)
```rust
// Source: pattern of scan_report / report_unattributed_schedules in run_api_wiring.rs
fn unpriced_reachable_models<'a>(
    has_ceiling: bool,
    prices: &PriceTable,
    reachable: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    if !has_ceiling { return Vec::new(); }
    reachable.into_iter().filter(|m| prices.row(m).is_none())
        .map(str::to_owned).collect::<std::collections::BTreeSet<_>>().into_iter().collect()
}
// call site, inside the Treasurer `else` branch after price_table() and before it moves into Arc:
// let models: Vec<String> = code_registry.list().iter().map(|(_, p)| p.data.model.clone()).collect();
// for m in unpriced_reachable_models(true, &price_table, models.iter().map(String::as_str)) {
//     log::warn!(target: PRICING_LOG_TARGET, "model {m} is reachable by a metered principal but has no \
//         treasurer.pricing row; agent-kind calls on it are refused at admission (422 model_unpriced) or \
//         halted at dispatch (model_unpriced); workflow assistants are not refused and graph-node models \
//         are not enumerated; add a treasurer.pricing row");
// }
```
Table test cases: no ceiling (empty), priced, unpriced, duplicates (one line), unsorted input (sorted output), empty registry.

### 4. Storage refusal and its table test
```rust
// Source: D-02 shape; one fixed message, no key material
fn unfinished(method: &str, backend: &str) -> RepositoryError {
    RepositoryError::QueryError(format!(
        "ContentListRepository::{method} is unfinished for the {backend} backend: content lists are not persisted"
    ))
}
fn get_by_id(&self, _id: Uuid) -> Result<Option<ContentList>, RepositoryError> { Err(unfinished("get_by_id", "sqlite")) }
// table test (in-module, feature-gated): call every one of the ten methods, assert Err(QueryError(m)) with m
// containing the method name and the backend; watched failing first (today each panics).
```

### 5. Bounded poll for the cancel test
```rust
// Source: replaces the 50 ms sleep in cancel_tests.rs local_cancel_signals_token
let tokens = pool.local_tokens();
while !tokens.is_registered(&run_id).await {
    tokio::time::sleep(Duration::from_millis(5)).await; // bounded by the 30 s outer timeout
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Crate-level `#![allow(rustdoc::…)]` for cross-crate links | Fully qualified paths or code spans, fixed where written | ADR-0033 amendment 2026-09-17 planned it for v0.11.0 | 474 diagnostics to clear now |
| `Unpriced` as `Failed` | `Halted` with a typed reason | Phase 55 | Matches ADR-0057's "a halt is a resume point, not a failure" |
| `Failed` row for a configuration incoherence | Halt row naming the model | Phase 55 | Webhook event changes from `failed` to `halted` for this case |

**Deprecated/outdated:** the CONTEXT line "thinking_effort_wire_value dead under default features" (already gated); the premise that TESTS-01 must precede FIXES-03.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `sqlx::mysql::MySqlPoolOptions::connect_lazy` exists in sqlx 0.8 and needs only a tokio context | Code Inventory 1 | The mysql table test needs a different construction (an in-module struct literal still works; the pool would need another source) |
| A2 | `sqlx migrate` checksums applied migration files, so a comment edit in `013_add_run_halt_reason.sql` is unsafe | Anti-Patterns | None if wrong; the file is frozen history either way |
| A3 | `cargo doc --message-format=json` carries machine-applicable suggestions for the 30 redundant links | Don't Hand-Roll | They are hand-edited instead (30 one-line edits) |
| A4 | A full `check-semver.sh` over twelve packages takes about 10-15 minutes here (46 s measured for the lightest package, `paladin-eval`) | Validation Architecture | The final plan's time budget is off; crates.io is reachable (index and static returned 200) |
| A5 | Serde internally tagged enums accept a struct variant `ModelUnpriced { model }` | Code Inventory 2 | A unit test at red-green time would show it immediately; the existing `AllowanceExhausted(AllowanceRefusal)` proves the tag mechanism |
| A6 | Stored agent-kind assistants can be listed at boot through `AssistantRepositoryPort::list` | Code Inventory 3 | The planner just documents them as not enumerated |

## Open Questions

1. **FIXES-05: does the locked D-06 conflict with the field's contract? (needs a decision before the executor flips the flag)**
   - What we know: D-06 says flip to `true` because the model reasons. The field is defined as "default call returned reasoning text"; the 50.1 row records `reasoning text: absent`; sibling rows with the same evidence are `false`. The recorded 41 tokens appear only on a non-default request and are token counts. D-06 also says the flag stays `false` if the records show "no reasoning tokens anywhere"; they do show 41.
   - What's unclear: whether the owner intends `reasoning` to widen to "can emit reasoning tokens", which would change the 300 s timeout for every model like it.
   - Recommendation: the first FIXES-05 task is a short decision checkpoint with the quoted lines. Option (a), follow D-06: set `true`, widen the `model_profile.rs` rustdoc to say so, and re-review sibling rows (haiku-5-5, opus 4.7) for the same evidence. Option (b): keep `false`, correct only the comment ("probe 1 returned no reasoning text", dropping the "listing truncated" reasoning), pin it, and record "verified, not wrong" in the evidence file. (b) honours the field's definition today and needs no sibling sweep; (a) is what the locked text asks for. This is a Claude-flagged tension, not a decision.

2. **D-08 "byte-for-byte" versus a delegating body**
   - What we know: the cheapest hermetic seam without a public surface change is a `pub(crate)` function holding the body, with `build_run_api_with_cadence` delegating.
   - What's unclear: whether "bodies byte-for-byte" is meant literally.
   - Recommendation: take the delegation (signature, behaviour and public surface unchanged, no API snapshot refresh). The literal reading needs a 250-line copy or a public sibling plus an API-surface update.

3. **Boot-warning coverage gaps beyond graph nodes**
   - What we know: D-07 excludes graph-node models. Stored agent-kind assistants (created via the assistants API, listable at boot) and runtime-provisioned agents are also not covered by `AgentRegistry::list()` at boot time.
   - Recommendation: say so in the warning and the Treasurer guide; optionally scan `assistant_repository.list` for agent-kind models at boot in the same pure function (small, follows the D-14 scan pattern) only if the plan has room.

4. **Fabricated item ids in `detect_collection_changes`**
   - What we know: D-03 says a method returning a fabricated value refuses with an existing `VersioningError`. `compare_collection_versions` is otherwise working and tested; refusing the whole method removes `detected_changes` for every caller. `ItemAdded(Uuid)`/`ItemRemoved(Uuid)` cannot carry "unknown" except as a nil UUID.
   - Recommendation: planner's call; the least destructive fence is to emit a nil UUID (a documented "item not tracked" sentinel) instead of `Uuid::new_v4()`, note it in rustdoc, and record the choice in the review file. Refusing the method is the literal D-03 reading. For `FieldVersionHistory.node_id` on a field with no versions, `Err(VersioningError::EntityNotFound(field_id))` is the honest existing variant; check the test near `field_version_service.rs:596` first.

5. **TESTS-01 before FIXES-03 is not binding**
   - The existing worker-level test is hermetic and exercises exactly the arm; FIXES-03 can land independently. Keep the roadmap order only if an end-to-end `build_run_api` check of the new status is wanted as an extra.

6. **Is `mock`-only part of the single-feature matrix?**
   - It yields ten warnings, all from `reasoning_defaults`, which cannot run without a provider. Recommendation: include it and gate the module on "a provider feature is on", so the matrix is simply every feature of `Cargo.toml`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | everything | yes | 1.97.1 | none needed |
| nightly-2026-09-20 | `make api-surface` | yes | pinned | none needed |
| python3 (`tomllib`) | the guard | yes | 3.13.16 | none needed |
| crates.io index and static (through the proxy) | `check-semver.sh` baselines, fresh downloads | yes (200 on both); builds here used `--offline` against the warm registry | n/a | `--offline` for builds |
| `cargo-semver-checks` | D-13 | yes | installed | none needed |
| `cargo-audit`, `cargo-deny` | `make security` | yes | installed | none needed |
| `shellcheck` | `make lint-shell` | yes | installed | none needed |
| `actionlint` | `ci.yml` step | **no** | n/a | CI Workflow Lint job; record `unrun-verify` |
| `cargo-llvm-cov` | the 82 % gate | **no** | n/a | CI `coverage` job; cannot be measured locally (record as an unrun measurement) |
| `mdbook` | doc build | **no** | n/a | not needed here |
| Docker daemon | integration with services | not usable (no daemon assumed per the brief) | n/a | in-memory and `sqlite::memory:` paths |
| Provider network (`api.openai.com`) | the two tests today | **blocked** (HTTP 000 through the proxy) | n/a | the scripted port |
| Provider API key | live probes | none | n/a | D-06 decides from recorded probes |

**Measurements not taken:** the 82 % coverage figure (no `cargo-llvm-cov`), a full `cargo test --workspace` run (only the facade lib binary, 1,474 tests in about 43 s, and targeted tests), the 20-run full-suite flake proof (only a 60-run isolated stress, 0 failures), a full twelve-package semver run (one package, 46 s), `make security`, `make doc-check`'s doctest step, and `actionlint`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest (`cargo test`), `tokio::test`, `serial_test` 3.2.0, plus bash harnesses in `tests/scripts/*_test.sh` |
| Config file | none; features per crate (`web-server` for the run API tests); `config.json` has `nyquist_validation: true` |
| Quick run command | `cargo test -p paladin-ai --features web-server --lib --offline <filter>` (facade rebuild about 1 min, tests under 1 s each) |
| Full suite command | `cargo test --workspace --no-fail-fast` (Phase 53 recorded 62 result lines and 7,774 passed; runtime not measured here) |
| Gate commands | `make check-gates` (about 1 s each script), `make test-shell-guards`, `make clean-code` (includes `make doc-check`: default bar about 4 min cold, all-features about 2 min, then `cargo test --workspace --doc`), `make api-surface`, `make openapi`, `make security`, `make check-semver` (`./scripts/check-semver.sh`) |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FIXES-01 | guard fails on a planted stub, passes on test/doc/string cases; 26 sites then 0 | shell harness | `./tests/scripts/check-no-stub-panics_test.sh` and `./scripts/check-no-stub-panics.sh` | Wave 0 (new) |
| FIXES-01 | every `ContentListRepository` method returns the typed `Err` | unit, feature-gated | `cargo test -p paladin-storage --features sqlite --offline content_list` and `--features mysql` | Wave 0 (new tests) |
| FIXES-02 | versioning fences, `ingest_content` refusal, no fabricated `expected_output`, prompt line | unit | `cargo test -p paladin-ai-core --offline base::`; `cargo test -p paladin-content --offline file_content_fetcher`; `cargo test -p paladin-ai --features web-server --lib --offline planning_service` | existing file edits plus new asserts |
| FIXES-02 | vocabulary freeze unaffected | integration | `cargo test -p paladin-ai --test entity_rename_guard --offline` | yes |
| FIXES-03 | Unpriced records `halted` with `model_unpriced` (row, SSE, webhook) | unit | `cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced` (renamed) | edit existing (`worker_tests.rs`) |
| FIXES-03 | `HaltReason::ModelUnpriced` serde, `as_str`, herald line, wire | unit and doctest | `cargo test -p paladin-ai-core --offline allowance::` and `--doc` | edit existing |
| FIXES-03 | OpenAPI description | golden | `cargo test -p paladin-web --lib --offline openapi_matches_committed_baseline` (`make openapi` to regenerate) | yes |
| FIXES-04 | re-home recorded (no code) | grep check | `rg -n "FIXES-04" .planning/REQUIREMENTS.md` shows the deferred entry and the traceability row | n/a |
| FIXES-05 | seed row matches the decision of Open Question 1 | unit | `cargo test -p paladin-llm --no-default-features --features anthropic --offline anthropic_opus_4_8_seed_matches_the_transcript` | edit existing |
| FIXES-06 | unpriced-model set (configured/unconfigured, no ceilings, priced/unpriced, duplicates) and rendered line | unit | `cargo test -p paladin-ai --features web-server --lib --offline unpriced_reachable` | Wave 0 (new) |
| TESTS-01 | two tests pass with no network | unit | `env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` | edit existing |
| TESTS-02 | registration wait; no flake | unit, loop | the `local_cancel_signals_token` test run 20 times under `cargo test -p paladin-ai --features web-server --lib` load (about 43 s per full-binary run) | edit existing |
| TESTS-03 | single-feature clippy clean | lint | the 13-combination loop of Code Inventory 8 (about 35 s per combination warm) | n/a |
| TESTS-04 | named lint sites | lint and doc | clippy matrix plus `cargo doc -p paladin-battalion -p paladin-llm --no-deps` | n/a |
| TESTS-05 | one provider-pair constant | unit | `cargo test -p paladin-llm --offline layers` plus a new same-slice test | Wave 0 (new test) |
| TESTS-06 | commander stubs point at the relocation | unit | `cargo test -p paladin-battalion --lib --offline test_error_handling test_partial_failure_handling` | edit existing |
| CIREL-08 | zero diagnostics under both bars | doc build | `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt` and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | n/a |

### Sampling Rate
- **Per task commit:** the narrow command from the map for that requirement, plus `cargo fmt --check` and `cargo clippy -p <touched crate>`.
- **Per wave merge:** `make test-shell-guards`, `make check-gates`, the touched crates' tests, and both rustdoc bars after any doc edit.
- **Phase gate:** `make clean-code`, `cargo test --workspace --no-fail-fast`, `make check-gates`, `make api-surface`, `make security`, `make check-migration-allowlist`, `make check-version-strings`, `make check-doc-config`, `./scripts/check-semver.sh` with its twelve lines recorded (expected `223 checks: 223 pass, 31 skip` each), and the manual credential review of every new log line and error message (no key value, model name only).

### Wave 0 Gaps
- [ ] `scripts/check-no-stub-panics.sh` and `tests/scripts/check-no-stub-panics_test.sh` (fixtures built under `mktemp -d`, a test-only root override variable like `VERSION_STRINGS_ROOT`); the red run against the real tree (26 lines) captured before the stub fixes
- [ ] feature-gated table tests in `sqlite_content_repository.rs` and `mysql_content_repository.rs` (watched failing: today each method panics)
- [ ] `unpriced_reachable_models` plus its renderer and table tests in `run_api_wiring.rs`
- [ ] a scripted-port double and the `pub(crate)` seam for the two hermetic tests
- [ ] a same-slice test for `SELF_HOSTED_PROVIDERS`
- [ ] crate changelog `## [Unreleased]` headings for `paladin-storage` and `paladin-llm` (and any other crate whose behaviour changes)

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so it is treated as enabled. The phase adds no route, credential, or outbound call; the relevant surface is error and log content.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | unchanged |
| V3 Session Management | no | unchanged |
| V4 Access Control | indirect | the new `halt_reason` is built only by `HaltReason::wire_json` and carries the model name, never a tenant, key name or key value (D-00g of the Treasurer phases); tenant-scoped run reads are unchanged |
| V5 Input Validation | yes | model names in the boot warning come from operator config, not request input; the storage refusal message is a fixed template with method and backend only |
| V6 Cryptography | no | none added |
| V7 Error Handling and Logging | yes | no key value, no backend error text outside `log::error!`; the boot warning names the model only; the guard script prints file paths and line numbers only |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A halt reason leaking another scope's figures or a key name | Information Disclosure | one wire builder (`HaltReason::wire_json`); the new variant carries only `model` |
| Panic from an unfinished stub taking down a worker | Denial of Service | typed `Err` (D-02) and the zero-exemption guard (D-01) |
| Unmetered spend on an unpriced model | Elevation / Repudiation | admission refuses (`422`), dispatch halts (D-04), the boot warning surfaces the gap (D-07); graph-node models remain an accepted, documented gap (WINDOWS row 67) |
| Network egress from a unit test | Tampering (supply chain) | scripted port, no key in the process (D-08) |
| Attacker-chosen model string growing a warn set | Denial of Service | the boot set is bounded by operator config; the settle-time set keeps its cap of 256 (`pricing.rs`) |

## Sources

### Primary (HIGH confidence)
- This repository at `a78338ddb`: every file path cited above was read in this session; counts come from commands run here (scanner, `cargo doc` with allows removed in a scratch worktree, 13 `cargo clippy` runs, targeted `cargo test` runs, `check-semver.sh --package paladin-eval`).
- `55-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md` (Blockers/Concerns), `.planning/WINDOWS.md` rows 56, 67, 73, `.planning/decisions/0033-cargo-doc-warning-bar.md` (amendment of 2026-09-18), the 50.1 and 51 probe records, `MIGRATION.md` §11.
- `.github/instructions/security.instructions.md` (credential and log review).

### Secondary (MEDIUM confidence)
- None; no external documentation was needed (no new libraries). Context7 and web search were not used.

### Tertiary (LOW confidence)
- Items tagged `[ASSUMED]` in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, nothing new is installed.
- Architecture: HIGH, every seam was read in source and the guard prototype ran against the real tree.
- Pitfalls: HIGH for the measured ones (31-vs-26, two-bar divergence, missing changelog anchors, stale `CARGO_MANIFEST_DIR`, `expected_output` prompt use); MEDIUM for the sqlx and rustdoc-JSON assumptions.
- FIXES-05 premise: the evidence is quoted and the conflict is real; the resolution is a human decision.

**Research date:** 2026-11-01
**Valid until:** 2026-11-15 (the doc-diagnostic counts drift with every doc edit; re-measure at plan time)
