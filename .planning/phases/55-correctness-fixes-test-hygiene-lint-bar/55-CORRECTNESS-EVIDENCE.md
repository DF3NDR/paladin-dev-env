# Phase 55 Correctness Evidence (plan 55-13)

Recorded 2026-11-01 on branch `claude/laughing-dirac-e0h2ax`. Phase head when the rows below were
re-run: `8202da48ccd718f91bf17f5a3bf67b092bb34162` (the 55-12 state-update commit). No source file
changed after it in this plan; the only commits that follow are planning records (this file,
`.planning/WINDOWS.md`, `.planning/STATE.md`). The workspace version is `0.11.0` (root `Cargo.toml`).

"Before" cells are copied from the plan SUMMARYs (55-01 .. 55-12), which measured them on each plan's
own start SHA before any edit (D-00e). "Head" cells were re-run for this file, in this session, on the
head above. Raw logs of this session's runs sit in the session scratchpad and are not committed; the
lines that matter are quoted here. No run in this file used a provider key, and none of them needed one.

## Prove-then-lock (D-00e)

| Item | Command | Before (from the plan SUMMARY) | Head output | Disposition |
|------|---------|--------------------------------|-------------|-------------|
| FIXES-01 | `./scripts/check-no-stub-panics.sh`; `cargo test -p paladin-storage --features sqlite --offline --lib content_list_repository_refuses`; the same with `--features mysql`; `grep -c 'todo!\|unimplemented!'` over the two adapter files | 55-01: guard red run, 26 finding lines in 3 files (see `## Guard red run (FIXES-01)`); the two table tests panicked at `sqlite_content_repository.rs:650` and `mysql_content_repository.rs:627` before the bodies changed | guard `OK: scanned 623 file(s) (19 test-only), 0 finding(s)`; sqlite `1 passed`; mysql `1 passed`; macro counts `0` and `0`; `content_filtering_service.rs` absent | changed in 55-01 |
| FIXES-02 | `grep -c 'unwrap_or_else(Uuid::new_v4)'` on `field_version_service.rs`; `grep -c Simplified` on `collection_versioning_service.rs`; `cargo test -p paladin-ai-core --offline --lib base::`; `cargo test -p paladin-content --offline --lib test_ingest_content_refuses`; `cargo test -p paladin-ai --features web-server --lib --offline planning_service` | 55-05: `.unwrap_or_else(Uuid::new_v4)` at `field_version_service.rs:487` and `ItemAdded(Uuid::new_v4())` / `ItemRemoved(Uuid::new_v4())` at `collection_versioning_service.rs:415,417`; 55-06: `test_ingest_content_refuses_valid_content_as_unfinished` failed with left `Ok(())`, and `create_plan_does_not_fabricate_an_expected_output` failed | counts `0` and `0`; `base::` `57 passed`; ingest refusal `1 passed`; `planning_service` `14 passed`; `INGEST_CONTENT_UNFINISHED` at `file_content_fetcher.rs:16`, returned at l.194; the old placeholder sentence appears in no `.rs` file | changed in 55-05 and 55-06 |
| FIXES-03 | `cargo test -p paladin-ai-core --offline --lib model_unpriced`; `cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced` | 55-02: the old test `..._fails_without_calling_the_llm` passed asserting `Failed`; with the new tests written first, `E0599: no variant named 'ModelUnpriced'` at `allowance.rs:1386` and `worker_tests.rs:2991` | allowance `3 passed` (`model_unpriced_serde_form_names_only_the_model`, `model_unpriced_herald_line_names_the_model`, `model_unpriced_wire_json_matches_the_serde_form`); worker `agent_kind_run_whose_model_became_unpriced_halts_without_calling_the_llm` `1 passed` | changed in 55-02 |
| FIXES-04 | `sed -n '/pub enum RunStatusValue/,/^}/p' crates/paladin-eval/src/scenario.rs`; `grep -n 'Cancelled' docs/schemas/eval-scenario.schema.json`; `cargo test -p paladin-eval --offline --test schema_golden` | 55-07: four variants (`Completed`, `Failed`, `Halted`, `AwaitingInput`), no `Cancelled`; `RunFinishStatus::Cancelled` at `trace.rs:174`; the schema diff touched two `"description":` lines only | still four variants; schema l.185 (`A cancelled run matches no value`) and l.463 (`Mirrors every status of ... RunFinishStatus except Cancelled`); `schema_golden` `1 passed` | re-homed (next minor); see `## Carry-forward` |
| FIXES-05 | the three source lines quoted in `## FIXES-05 verification`; `grep -n 'with_reasoning' crates/paladin-llm/src/registry/seeded.rs` | 55-04: probe 1 `reasoning text: absent`; effort probe row 1 `0` reasoning tokens; contract sentence in `model_profile.rs` | all three lines re-read from the files on the head; `seeded.rs` keeps `.with_reasoning(false)` for `claude_opus_4_8` and cites both probes; `anthropic_opus_4_8_seed_matches_the_transcript` `1 passed` | verified, not wrong |
| FIXES-06 | `cargo test -p paladin-ai --features web-server --lib --offline unpriced_`; `... stored_agent_model_scan` | 55-09: before the functions existed the tests failed to compile (`cannot find type PriceTable`, `StoredAgentModelScan`); WINDOWS row 67 `open` | `unpriced_` `11 passed`, including `unpriced_reachable_models_table`, `unpriced_model_warning_line_names_the_model_and_the_gaps` and `boot_with_an_unpriced_reachable_model_under_an_allowance_still_succeeds`; `stored_agent_model_scan` `2 passed`; `unpriced_reachable_models` l.780, `unpriced_model_warning_line` l.805, `report_unpriced_reachable_models` l.907 of `run_api_wiring.rs` | changed in 55-09 |
| TESTS-01 | `env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy -u OPENAI_API_KEY cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` | 55-09: `test result: FAILED. 0 passed; 2 failed ... finished in 20.74s` (`the run never reached a terminal status`) | `test result: ok. 2 passed; 0 failed; ... finished in 1.40s`; `OPENAI_API_KEY` appears 0 times inside `persisted_trace_records` | changed in 55-09 |
| TESTS-02 | `cargo test -p paladin-ai --features web-server --lib --offline is_registered_tracks_register_and_remove`; `... local_cancel_signals_token`; the full facade lib binary | 55-03: fixed `sleep(50ms)` at `cancel_tests.rs:362`; `E0599 no method named is_registered` before the inspector existed | `is_registered_tracks_register_and_remove` `1 passed`; `local_cancel_signals_token` `1 passed`; full binary `1490 passed; 0 failed` (see `## Hermetic and flake proofs`) | changed in 55-03 |
| TESTS-03 | the 13-combination `cargo clippy -p paladin-llm --tests --offline [flags] -- -D warnings` loop | 55-04: warning counts per combination 1, 5, 7, 4, 6, 8, 5, 5, 5, 7, 10, 5, 5 | 13 exits of `0`, 13 warning counts of `0` (see `## Lint and rustdoc bars`); `git diff dea69385 -- crates/paladin-llm/src` adds 0 `#[allow(`/`#[expect(` lines | changed in 55-04 |
| TESTS-04 | `cargo doc -p paladin-battalion --no-deps --offline` then `grep -c '^warning'`; the `reasoning.rs` redundant-link grep on `cargo doc -p paladin-llm`; `grep -c thinking_effort_wire_value` over the 13 clippy logs | 55-04: three of the four sites fixed in 51-14 (the 51-14 record is `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/deferred-items.md`, "Resolved in plan 51-14"); the fourth (unused `ReasoningFallback` import) fixed in 55-04 | `0`; `0`; `0` in every log (the function is defined at `conformance.rs:297` and used by the `ollama`, `openai-compatible` and `kimi` adapters, gated on those three features); the default build's `ReasoningFallback` warning is gone (default combination `0`) | already closed (51-14) for three sites; changed in 55-04 for the fourth |
| TESTS-05 | `grep -rn '\["ollama", "openai-compatible"\]' crates/paladin-llm/src`; `cargo test -p paladin-llm --offline --lib self_hosted_pair`; `... wire_refusal_points_self_hosted` | 55-04: three literal pairs (`reasoning.rs:257`, `config/layers.rs:342`, a test loop at `reasoning.rs:700`); the layering test failed to compile (`E0603: constant SELF_HOSTED_PROVIDERS is private`) | exactly one hit, `reasoning.rs:263` (`pub(crate) const SELF_HOSTED_PROVIDERS`); `reasoning_wire_providers_are_the_self_hosted_pair` `1 passed`; `wire_refusal_points_self_hosted_providers_at_reasoning_wire` `1 passed` | changed in 55-04 |
| TESTS-06 | `grep -c commander_error_paths_test crates/paladin-battalion/src/commander.rs`; `grep -c TODO` on the same file; `cargo test -p paladin-battalion --offline --lib test_error_handling`; `cargo test -p paladin-ai --test lib --offline commander_error_paths` | 55-03: four to-do blocks at `commander.rs` l.3834, 3870, 3917, 3957 | pointer count `5`; `TODO` count `0`; `test_error_handling` `3 passed`; relocated tests `4 passed` | changed in 55-03 |
| CIREL-08 | `cargo doc --workspace --no-deps --offline`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --offline`; `grep -rnE 'allow\(rustdoc\|expect\(rustdoc' crates/*/src src` | 55-08, 55-10, 55-11: 474 default-bar and 455 all-features-bar diagnostics across five crates, behind eight crate-level suppressions | default bar prints no `warning:` line (`0`); all-features bar exit `0`; suppression grep prints `0` hits (see `## Lint and rustdoc bars`) | changed in 55-08, 55-10 and 55-11 |
| Scout: `HaltReason` non-exhaustive | `grep -B2 'pub enum HaltReason' crates/paladin-core/src/platform/container/allowance.rs` | 55-02: "still `#[non_exhaustive]`" after the variant was added | `#[serde(tag = "reason", rename_all = "snake_case")]` then `#[non_exhaustive]` then `pub enum HaltReason {` | re-confirmed; the 55-02 variant is an additive change on a non-exhaustive enum |
| Scout: `RunFinishStatus` non-exhaustive | `grep -n 'pub enum RunFinishStatus' -B3 crates/paladin-core/src/platform/container/trace.rs` | 55-07: `RunFinishStatus::Cancelled` at `trace.rs:174` | `#[non_exhaustive]` at l.164, `pub enum RunFinishStatus {` at l.165 | re-confirmed, not touched by the phase |
| Scout: `RepositoryError` exhaustive | `grep -B3 'pub enum RepositoryError' crates/paladin-ports/src/output/repository_port.rs` | 55-01: the refusals reuse the existing `QueryError`; `git diff` empty for `crates/paladin-ports` | `#[derive(Debug, Clone, Error)]` then `pub enum RepositoryError {`, no `#[non_exhaustive]` | re-confirmed; no variant was added, which is why plan 55-01 used `QueryError` |
| Scout: `RunStatusValue` exhaustive | `grep -B3 'pub enum RunStatusValue' crates/paladin-eval/src/scenario.rs` | 55-07: four variants and no `#[non_exhaustive]` before any edit | `#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]`, `#[serde(rename_all = "snake_case")]`, `pub enum RunStatusValue {`, no `#[non_exhaustive]` | re-confirmed; this is why adding `Cancelled` is a break and FIXES-04 is re-homed |
| Scout: `Assertion` exhaustive | `grep -B3 'pub enum Assertion' crates/paladin-eval/src/scenario.rs` | 55-07 (same crate, same scout) | `#[derive(Debug, Clone, JsonSchema, PartialEq)]`, `#[serde(rename_all = "snake_case", deny_unknown_fields)]`, `pub enum Assertion {`, no `#[non_exhaustive]` | re-confirmed |
| Scout: `VersioningError` exhaustive | `grep -rn 'pub enum VersioningError' -B3 crates/paladin-core/src/base/` | 55-05: `git diff` of `entity_version_service.rs` empty, no new variant | `#[derive(Debug, Clone, Error)]` then `pub enum VersioningError {` at `entity_version_service.rs:30`, no `#[non_exhaustive]` | re-confirmed; no variant added, so the field fence reuses `EntityNotFound` |
| Scout: `AgentRegistry::list` exists | `grep -n 'pub fn list' crates/paladin-web/src/agent_registry.rs` | 55-09 relied on it for the boot walk of code-registered agents | `190:    pub fn list(&self) -> Vec<(String, Arc<Paladin>)> {` | re-confirmed (the boot warning reads it) |
| Scout: `AssistantRepositoryPort::list` exists | `grep -n 'async fn list' crates/paladin-ports/src/output/assistant_repository_port.rs` | 55-09 pages it for stored agent-kind models (`ASSISTANT_SCAN_PAGE`) | `347:    async fn list(` (the trait method; l.216 is the doc example) | re-confirmed (the boot scan reads it) |
| Scout: `connect_lazy` exists | `grep -rn 'connect_lazy' src crates --include=*.rs` | 55-01: the MySQL refusal test builds its store without a server | `mysql_content_repository.rs:810` (comment: "contacts no server until a query runs") and `:813` (`.connect_lazy("mysql://localhost/paladin_unused")`) | re-confirmed |
| Scout: the eval schema carries rustdoc descriptions | `grep -n 'Cancelled\|cancelled' docs/schemas/eval-scenario.schema.json` | 55-07: two `"description":` lines changed, nothing else | description of `Assertion::RunStatus` at l.185 and of `RunStatusValue` at l.463, both naming `Cancelled`; `schema_golden` `1 passed` | re-confirmed; the descriptions are generated from the rustdoc, so the golden moves with it |

## Guard red run (FIXES-01)

Copied verbatim from `55-01-SUMMARY.md`: run on the real tree after the guard was committed and before any
stub was touched (`./scripts/check-no-stub-panics.sh; echo "exit=$?"`), at plan-start SHA
`7f2eabe93cb32bccf3da1eb3121172a382193525` plus the guard commit `75f3c05a2`.

```
exit=1
🔍 Checking published crates for stub panics outside #[cfg(test)] ...
❌ Stub-panic check failed (FOUND)

crates/paladin-content/src/services/content_filtering_service.rs:21: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:22: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:23: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:24: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:25: todo! outside #[cfg(test)] in a published crate
crates/paladin-content/src/services/content_filtering_service.rs:26: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:627: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:631: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:635: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:639: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:643: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:651: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:659: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:663: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:667: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/mysql_content_repository.rs:671: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:650: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:654: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:658: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:662: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:666: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:674: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:682: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:686: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:690: todo! outside #[cfg(test)] in a published crate
crates/paladin-storage/src/sqlite_content_repository.rs:694: todo! outside #[cfg(test)] in a published crate
scanned 624 file(s) (19 test-only), 26 finding(s)

If this failure is unexpected:
  1. Replace each reported stub with a typed Err (or delete the dead code);
     a stub macro outside #[cfg(test)] in a published crate panics a caller.
  2. This gate has no exemption list, no inline marker and no environment
     switch (D-01); do not add one to silence a finding.
```

26 finding lines: 6 (the never-compiled `content_filtering_service.rs`) + 10 (MySQL) + 10 (SQLite), in 3 files.

Head run on `8202da48c`, re-run for this file:

```
$ ./scripts/check-no-stub-panics.sh; echo "exit=$?"
🔍 Checking published crates for stub panics outside #[cfg(test)] ...
✅ OK: scanned 623 file(s) (19 test-only), 0 finding(s)
exit=0
```

The one file fewer than the red run is the deleted `content_filtering_service.rs`.

## Hermetic and flake proofs (TESTS-01, TESTS-02)

**No-network run (TESTS-01).** `touch src/lib.rs` first (RESEARCH Pitfall 6), then, on the head, with every proxy
variable and `OPENAI_API_KEY` unset by `env -u`:

```
$ env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy -u OPENAI_API_KEY cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default ... ok
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1488 filtered out; finished in 1.40s
```

Before (55-09, plan-start SHA `bcd78e3d3`, same command without `-u OPENAI_API_KEY`): `test result: FAILED. 0 passed; 2 failed; ... finished in 20.74s`, both at
`the run never reached a terminal status`. The two tests went from a 20.74 s failure to a 1.40 s pass; the scripted `CannedAnswerPaladinPort`
enters through the crate-private `build_run_api_with_engine_port` seam, so the production router and worker pool run and no model call leaves the process.

**20-run flake proof (55-03, carried).** Plan-start SHA `30ad786bac1548d3f34f793059872e5897613677`. After `touch src/lib.rs` and one
`--no-run` build, with default parallel test threads and `CARGO_INCREMENTAL=0`:

```
for i in $(seq 1 20); do cargo test -p paladin-ai --features web-server --lib --offline -- --skip build_run_api_persists 2>&1 | grep -E '^test result:|local_cancel_signals_token'; done
```

- `local_cancel_signals_token ... ok` lines: 20 of 20; `test result: ok` lines: 20 of 20, zero `FAILED`.
- Every run: `test result: ok. 1480 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in <t>`, with `<t>` between 21.71 s and 23.01 s.
- The `--skip` excluded only the two network-bound tests that plan 55-09 later made hermetic.

**Full facade lib binary, no `--skip` (head, this session).** One run, default parallel threads:

```
$ cargo test -p paladin-ai --features web-server --lib --offline
test application::services::run::cancel_tests::local_cancel_signals_token ... ok
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default ... ok
test infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set ... ok
test result: ok. 1490 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.73s
```

The head binary holds more tests than 55-03's loop (1480 run plus 2 skipped) because plans 55-06 and 55-09 added tests to the facade after that loop ran; the figure is a head measurement, not a delta claim. This full run had the sandbox's proxy variables set (the `env -u` run above is the hermetic proof); the two trace tests passed regardless. The two tests are in the unskipped binary and pass, and so does the cancel test.

## Lint and rustdoc bars (TESTS-03, TESTS-04, CIREL-08)

**TESTS-03: 13-combination `paladin-llm` clippy loop, head `8202da48c`.** Command per combination:
`cargo clippy -p paladin-llm --tests --offline [--no-default-features --features <f>] -- -D warnings`, warning count taken as `grep -c '^warning:'` over the log.

| Combination | Before (55-04, plan-start `dea69385`) | Head exit | Head warnings |
|-------------|---------------------------------------|-----------|---------------|
| default | 1 | 0 | 0 |
| openai | 5 | 0 | 0 |
| anthropic | 7 | 0 | 0 |
| deepseek | 4 | 0 | 0 |
| kimi | 6 | 0 | 0 |
| qwen | 8 | 0 | 0 |
| grok | 5 | 0 | 0 |
| ollama | 5 | 0 | 0 |
| openai-compatible | 5 | 0 | 0 |
| gemini | 7 | 0 | 0 |
| mock | 10 | 0 | 0 |
| vision | 5 | 0 | 0 |
| openai-embeddings | 5 | 0 | 0 |

13 exits of `0`. `git diff dea69385 -- crates/paladin-llm/src | grep '^+' | grep -cE '#\[(allow|expect)\('` prints `0`: the bar is met by cfg gates, not suppression.

**TESTS-04 proving commands (from 55-04, re-run on the head).**

| Command | 55-04 result | Head |
|---------|--------------|------|
| `cargo doc -p paladin-battalion --no-deps --offline 2>&1 \| grep -c '^warning'` | `0` (the `NODE_CACHE_KEY_VERSION` link) | `0` |
| `cargo doc -p paladin-llm --no-deps --offline 2>&1 \| grep -A3 'redundant explicit link' \| grep -c 'reasoning.rs'` | `0` | `0` |
| `grep -c thinking_effort_wire_value` over the 13 clippy logs | `0` in all 13 before and after | `0` in all 13 |

**CIREL-08: both rustdoc bars on the head.**

```
$ cargo doc --workspace --no-deps --offline            -> exit 0, `grep -c 'warning:'` over the log: 0
$ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --offline   -> exit 0, 13 crates documented, no diagnostic
$ grep -rnE 'allow\(rustdoc|expect\(rustdoc' crates/*/src src | wc -l   -> 0
$ cargo doc -p paladin-ai-core --no-deps --offline 2>&1 | grep -c 'warning:'   -> 0
```

The last line is the measurement that WINDOWS row 56 (`paladin-ai-core`, `cost.rs`) waits on; see `## Carry-forward`.

Per-crate before and after, from 55-08, 55-10 and 55-11 (each crate measured at its plan start with its crate-level suppressions removed in the working tree):

| Crate | Default before | All-features before | Classes | After (both bars) | Commit |
|-------|----------------|---------------------|---------|-------------------|--------|
| `paladin-ports` | 187 | 187 | 170 unresolved + 17 redundant | 0 | `309774be8` |
| `paladin-llm` | 117 | 105 | unresolved | 0 | `8ea9f1357` |
| `paladin-storage` | 27 | 28 | unresolved | 0 | `d53d4233c` |
| `paladin-notifications` | 3 | 0 | unresolved | 0 | `23b6426d8` |
| `paladin-ai` (facade) | 140 | 135 | 127 + 13 / 122 + 13; 0 `invalid_html_tags` | 0 | `14ee125e7` |
| Total | 474 | 455 | | 0 | |

Eight crate-level suppressions removed in total (two in `paladin-ports`, one each in `paladin-llm`, `paladin-storage`, `paladin-notifications`, three in the facade); none remains.

## FIXES-05 verification

D-06 as reconciled (55-CONTEXT "Research reconciliation"): `claude-opus-4-8` keeps `reasoning: false` because no default-call row shows reasoning text. The three source lines, re-read on the head from the two probe files and `model_profile.rs`:

1. `.planning/milestones/v0.11.0-phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md`, `### Probes: claude-opus-4-8` (l.878), row 1, `default parameters`: `| 1 | default parameters | - | ACCEPTED (200) | end_turn | non-empty | ... | absent |`. The last column is `reasoning text`, and it reads `absent`.
2. `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md`, `### claude-opus-4-8` (l.881): `| 1 | default | - | 200 | end_turn | 0 | 37 | 1505 | accepted | - |`. Rows 2-6 (`output_config` low to max) and rows 7-10 (`output_config+adaptive` low to xhigh) all carry `0` reasoning tokens; row 11 (`output_config+adaptive`, `max`) is `| 41 | 44 |`; `thinking_object` rows 12 and 13 (`off`, `auto`) carry `0`.
3. `crates/paladin-ports/src/output/model_profile.rs:963`: `/// Whether the default call to this model **returned reasoning text**.`

The 41 reasoning tokens sit on a non-default request (`max` with `thinking: adaptive`), outside the field's stated contract ("the default call"). `crates/paladin-llm/src/registry/seeded.rs` `claude_opus_4_8` keeps `.with_reasoning(false)` and its comment cites probe 1 and the `### claude-opus-4-8` effort table. No live probe and no key were used for this verification.

FIXES-05 verified against both probe records, flag not wrong

## WINDOWS rows 73 and 67 (D-07, D-08)

Both rows were dispositioned only after their closing evidence was re-run on the head `8202da48c` in this session.

| Row | Closing evidence re-run on the head | Disposition |
|-----|-------------------------------------|-------------|
| 73 (phase 50, `run_api_wiring.rs`, the two network-bound tests) | the no-network command in `## Hermetic and flake proofs`: `test result: ok. 2 passed; 0 failed; ... finished in 1.40s`; the unskipped facade binary also passes both | `node .claude/gsd-core/bin/gsd-tools.cjs windows fixed 73`, then the reason cell set to cite `build_run_api_persists_run_traces_when_trace_persist_is_set`, `build_run_api_persists_no_run_traces_by_default`, the crate-private `build_run_api_with_engine_port` seam and this file |
| 67 (phase 42, `settlement.rs`, the unpriced graph-node gap) | `cargo test -p paladin-ai --features web-server --lib --offline unpriced_reachable_models_table`: `test result: ok. 1 passed; 0 failed`; the wider `unpriced_` filter `11 passed` and `stored_agent_model_scan` `2 passed` (see the FIXES-06 row) | `node .claude/gsd-core/bin/gsd-tools.cjs windows waive 67 "<reason>"`, citing milestone decision B4 (the operator contract stands: price every model a metered principal can reach), the boot warning of plan 55-09 and the accepted graph-node contract in `treasurer.md` |

The reason cell of row 73 was set by hand after `windows fixed 73`, because that subcommand takes no reason argument (rows 58, 62 and 63 were closed the same way). The edit was made in both places the ledger stores it, the table row and the JSON entry, and the file was then loaded with the tool's own `parseLedger` and re-rendered with `renderLedger`: the output is byte-identical to the file on disk.

Counters, from `windows status`:

| Counter | Plan start (`8202da48c`) | After |
|---------|--------------------------|-------|
| `open_count` | 15 | 13 |
| `waived_count` | 39 | 40 |
| `fixed_count` | 33 | 34 |
| `total_count` | 87 | 87 |

The entries tally agrees with the frontmatter (13 open, 40 waived, 34 fixed, 87 total). `git diff 8202da48c -- .planning/WINDOWS.md` touches the frontmatter and rows 67 and 73 only, in the table and in the JSON block.

## Carry-forward

**Phase 57 (register sweep).**

- WINDOWS row 56 (`paladin-ai-core`, `cost.rs`, rustdoc `broken_intra_doc_links`) is fixed in fact and left unchanged here (PD-13-1; it is outside this phase's requirements). Measurement on the head:
  `cargo doc -p paladin-ai-core --no-deps --offline 2>&1 | grep -c 'warning:'` prints `0`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --offline` exits `0`; the whole workspace is clean under both ADR-0033 bars with no suppression left. Phase 57 can close the row on this measurement.
- Rows 83 and 84 (`actionlint` unrun on the 52-01 and 52-06 `ci.yml` edits) stay `open` and await the CI `Workflow Lint` job. That job now also covers the `ci.yml` step added in 55-01 (`Check no stub panics in published crates (FIXES-01)`), which was likewise checked locally only by YAML parsing. No new row was opened for it (55-01 recorded the unrun check in its SUMMARY under PD-01-3).
- Rows 85-87 (53-REVIEW WR-01..03, `thread_controller.rs` and `principal.rs`) are untouched and deferred to a `56.x` decimal phase, or `/gsd-phase edit 55`, per CONTEXT and `deferred-items.md` (they are code fixes outside FIXES-01..06).
- The other CONTEXT `<deferred>` items (News API key in a URL, `${...}` in the treasurer webhook `secret`) stay with the same `56.x` phase; `deferred-items.md` holds each with its proposed home.

**Phase 58 (pre-tag audit).** Crate changelogs without an `## [Unreleased]` or `## [0.11.1]` anchor, re-measured on the head:

```
$ for f in crates/*/CHANGELOG.md; do grep -qE '^## \[(Unreleased|0\.11\.1)\]' "$f" || echo "$f"; done
crates/paladin-battalion/CHANGELOG.md
crates/paladin-eval/CHANGELOG.md
crates/paladin-herald/CHANGELOG.md
crates/paladin-llm/CHANGELOG.md
crates/paladin-notifications/CHANGELOG.md
crates/paladin-ports/CHANGELOG.md
```

Six of eleven, as expected. The five that carry an anchor are `paladin-content`, `paladin-core`, `paladin-memory`, `paladin-storage` and `paladin-web`. A scratch run of `scripts/finalize-crate-changelogs.sh` in 55-12 proved that a changelog with an `[Unreleased]` heading finalizes; the six above need the heading before the 0.11.1 finalize step.

**Next minor.**

- FIXES-04: `RunStatusValue::Cancelled` plus the matching `run_status_matches` arm, then `UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden`, and replace the rustdoc and guide notes that name the gap (`RunStatusValue` is exhaustive, so adding the variant is a break; see the scout row).
- The base-layer finish or retire options A, B and C in `55-BASE-LAYER-REVIEW.md` (every item is kept and fenced this milestone).
- The graph-node pricing check: a registration-time coverage check needs a listing method on `AssistantResolver` (the row 67 waiver text).
- Widening `ModelProfile.reasoning` beyond the default-call contract (deferred by D-06 as reconciled; see `## FIXES-05 verification`).
