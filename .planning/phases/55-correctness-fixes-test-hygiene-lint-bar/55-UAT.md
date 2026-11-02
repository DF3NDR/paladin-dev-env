---
status: testing
phase: 55-correctness-fixes-test-hygiene-lint-bar
source: [55-01-SUMMARY.md, 55-02-SUMMARY.md, 55-03-SUMMARY.md, 55-04-SUMMARY.md, 55-05-SUMMARY.md, 55-06-SUMMARY.md, 55-07-SUMMARY.md, 55-08-SUMMARY.md, 55-09-SUMMARY.md, 55-10-SUMMARY.md, 55-11-SUMMARY.md, 55-12-SUMMARY.md, 55-13-SUMMARY.md]
started: 2026-11-02T01:40:47Z
updated: 2026-11-02T01:40:47Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 1
name: Stub-panic gate holds on the real tree and fires on a planted stub
expected: |
  `scripts/check-no-stub-panics.sh` on the tree prints `OK: scanned ... file(s) (... test-only), 0 finding(s)` and exits 0; `make check-gates` runs it. Planting `pub fn planted() -> u8 { todo!() }` in a published crate's non-test code (or a scratch copy via `STUB_PANICS_ROOT`) makes it exit 1 and name the file:line.
awaiting: user response

## Tests

### 1. Stub-panic gate holds on the real tree and fires on a planted stub
expected: `scripts/check-no-stub-panics.sh` on the tree prints `OK: scanned ... file(s) (... test-only), 0 finding(s)` and exits 0; `make check-gates` runs it. Planting `pub fn planted() -> u8 { todo!() }` in a published crate's non-test code (or a scratch copy via `STUB_PANICS_ROOT`) makes it exit 1 and name the file:line.
result: [pending]

### 2. An unpriced model at dispatch halts the run instead of failing it
expected: With `treasurer.allowance` configured, an agent-kind run whose model has no `treasurer.pricing` row at dispatch ends `halted` (not `failed`), with `halt_reason` `{"reason":"model_unpriced","model":"<name>"}`, no error, SSE `done` with `status: halted`, a `halted` webhook, and no LLM call. Admission-time `422 model_unpriced` is unchanged. Docs (Platform API, Treasurer guide, MIGRATION M-B-31) say to add the pricing row and resubmit.
result: [pending]

### 3. Boot warns once per unpriced reachable model and still boots
expected: With a Treasurer configured and an agent-kind assistant (or code-registry agent) on a model with no `treasurer.pricing` row, `paladin-server` boots normally and logs exactly one `warn!` under `paladin::pricing` per such model: "model `<name>` is reachable by a metered principal but has no treasurer.pricing row: ...". Workflow assistants are never refused.
result: [pending]

### 4. `cargo test` passes with no network
expected: In a sandbox with no network (e.g. `unshare -rn` or proxies unset), `cargo test` passes, including `run_api_wiring::tests::build_run_api_persists_*`, which now use a scripted port — no API key, no network, no serial group.
result: [pending]

### 5. Cancel test is non-flaky under repeated parallel runs
expected: `cancel_tests::local_cancel_signals_token` waits on a registration signal (no fixed sleep) and passes on every one of repeated runs inside the full `application::services::run::` filter under parallel load.
result: [pending]

### 6. `paladin-llm` single-feature clippy is warning-free
expected: `cargo clippy -p paladin-llm --tests --no-default-features --features <each single feature> -- -D warnings` (and default features) is clean for all combinations, with cfg gates and no new `allow`.
result: [pending]

### 7. Content-processing stubs refuse instead of faking results
expected: `FileContentFetcher::ingest_content` returns an `Err` saying ingestion is unfinished and nothing was stored (URL validation errors still come first). Autonomous planning (`MaxLoops::Auto`) builds subtasks with an empty `expected_output`, and the subtask prompt omits the `EXPECTED OUTPUT` line; a caller-supplied expected output renders exactly as before.
result: [pending]

### 8. Base-layer rustdoc says each retained item is unfinished
expected: The rustdoc on `Field`, `CollectionType`/`Item`, `element`, `base/mod.rs`, `FieldVersionService` and `CollectionVersionService` reads accurately and sufficiently to a reader: each says it is unfinished scaffolding and names `55-BASE-LAYER-REVIEW.md`. (55-05 D3 — human judgment; the vocabulary freeze, `cargo doc` 0 warnings and doctests already pass.)
result: [pending]

### 9. FIXES-04 re-home is visible to an eval author
expected: An author trying `run_status: cancelled` finds why it is not supported until the next minor in three places: the `RunStatusValue` rustdoc, the JSON schema description and the `docs/src/user-guides/eval-harness.md` table row. No public eval enum changed.
result: [pending]

### 10. `doc-coverage-report.md` reads as an adequate release-review record
expected: The regenerated page reads as a dated measurement record at `14ee125e7`: the two bars and their commands, the twelve `missing_docs` roots and the eight removed suppressions, with no >= 90% figure. (55-11 D3 — reader judgment; `make check-doc-config` and `make check-version-strings` pass.)
result: [pending]

### 11. Release records cover every Phase 55 behaviour change
expected: CHANGELOGs (incl. a new `paladin-storage` `[Unreleased]` anchor), `MIGRATION.md` rows M-B-30..M-B-35, the section 11.6 paragraph and section 11.8 step 6 read correctly to a consumer/operator, and `55-BASE-LAYER-REVIEW.md` gives a keep/fence decision with evidence for each of the 8 items.
result: [pending]

### 12. An unversioned field's history is EntityNotFound; purge returns Ok(0)
expected: A field with no stored versions is EntityNotFound for its history (service and repository) and Ok(0) for a purge
result: pass
source: automated
coverage_id: 55-05/D1

### 13. Collection count changes carry the nil sentinel
expected: Collection count changes carry the nil sentinel, comparisons are idempotent, equal / same-length-different / empty item lists report exactly the documented changes
result: pass
source: automated
coverage_id: 55-05/D2

### 14. Rustdoc is warning-free with no suppressions
expected: Facade rustdoc diagnostics fixed (140 default / 135 all-features to 0) and its three crate-level suppressions removed; no rustdoc allow or expect remains in crates/*/src or src
result: pass
source: automated
coverage_id: 55-11/D1

### 15. ADR-0033 amended in place
expected: ADR-0033 amended in place with the 2026-11-01 block (additions only)
result: pass
source: automated
coverage_id: 55-11/D2

## Summary

total: 15
passed: 4
issues: 0
pending: 11
skipped: 0
blocked: 0

## Gaps

[none yet]
