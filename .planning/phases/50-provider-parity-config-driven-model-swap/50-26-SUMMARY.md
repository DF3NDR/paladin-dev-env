---
phase: 50-provider-parity-config-driven-model-swap
plan: 26
subsystem: llm
tags: [rust, paladin-ports, paladin-llm, llm-error, billed-usage, treasurer, semver, migration, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-25 LlmError::empty_completion / LlmError::refusal and the seven shape-proofed consumer files; 50-21/50-23 map_compat_usage returning the folded TokenUsage plus the vendor total; 50-18 measured-registration pattern (D-27)"
provides:
  - "paladin_ports::output::llm_port::BilledUsage { model, usage, cost } (#[non_exhaustive], Debug/Clone/PartialEq) with BilledUsage::new and with_cost"
  - "LlmError::EmptyCompletion { detail, billed } and LlmError::Refusal { provider, model, detail, billed }, both #[non_exhaustive], billed: Option<Box<BilledUsage>>; Display and Permanent unchanged"
  - "LlmError::with_billed_usage (the two variants only; reaches AllProvidersFailed.last; any other variant unchanged) and LlmError::billed_usage (looks through AllProvidersFailed)"
  - "pub(crate) fn response_guards::bill(err, model, Option<TokenUsage>)"
  - "Compat engine buffered generate maps usage before the guard and bills with the served model (else the request model); CompatResponse.usage is Option"
  - "Tracer test compat_billed_empty_completion_carries_its_usage; tests billed_usage_new_and_with_cost_round_trip, with_billed_usage_sets_the_bill_on_empty_completion_and_refusal, billed_usage_looks_through_all_providers_failed_to_its_last_error, with_billed_usage_leaves_other_variants_unchanged, bill_attaches_reported_usage_only"
  - "Registration (same commit): enum_tuple_variant_changed_kind = allow in crates/paladin-ports/Cargo.toml; the paladin-ports | LlmError 9.2 row extended; allowlist entry (new pair) plus the enum_variant_marked_non_exhaustive justification extended; MIGRATION 9.8 DR-17; consumer muster pin; CHANGELOG entries (root, paladin-ports, paladin-llm)"
affects: [50-27, 50-28, 50-29, 50-30, 50-31]

tech-stack:
  added: []
  patterns:
    - "A billed-but-failed call's usage rides on the typed error (billed slot plus accessor), never on the message text"
    - "An adapter maps usage before the shared guard and routes the guard's error through response_guards::bill"
    - "A detail-annotating helper edits the variant's detail in place (if let .. = &mut err) so any other field, the bill included, survives"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-llm/src/response_guards.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-ports/Cargo.toml
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - tests/consumer_surface_muster.rs
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md

key-decisions:
  - "billed is Option<Box<BilledUsage>> on both variants (A1-50-26: boxed for large_enum_variant); BilledUsage holds a model id, TokenUsage and Option<Cost> only"
  - "No usage object means no bill (D-00c); CompatResponse.usage became Option so such a body reaches the guard, while a successful body without usage keeps the schema-mismatch ProcessingError"
  - "The billed model is the body's served model when non-empty, else the request model (A3-50-26)"
  - "D-27 measured: EmptyCompletion fires enum_tuple_variant_changed_kind (newly allowed) and enum_variant_marked_non_exhaustive (already allowed); Refusal and BilledUsage fire nothing"

patterns-established:
  - "Billing a failure: map usage, classify, then .map_err(|e| bill(e, served_or_request_model, usage))"

requirements-completed: [PARITY-05, PARITY-07]

duration: 227min
completed: 2026-10-18
---

# Phase 50 Plan 26: LlmError carries the usage of billed empty and refused calls; break registered (G-50-5a) Summary

**`LlmError::EmptyCompletion` and `LlmError::Refusal` are now `#[non_exhaustive]` struct variants with a typed
`billed: Option<Box<BilledUsage>>` slot, read through `billed_usage()`.**

- A compat call answering `finish_reason: length` with empty content and usage 1_000 / 62_000 / 63_000 now returns
  `Err(EmptyCompletion { .. })`. Its `billed_usage()` carries `kimi-served-test`, total 63_000 and `cost: None`.
- With no usage object, `billed_usage()` is `None`.
- The break is measured, and it is registered in the same commit as the code.

## Performance

- **Duration:** about 227 min. Nearly all of it was build time. With `CARGO_INCREMENTAL=0` (needed for disk), each
  `--workspace --all-targets --all-features` build took about 40 min.
- **Started:** 2026-10-18T15:28:16Z
- **Completed:** 2026-10-18T19:15:20Z
- **Tasks:** 2 of 2, committed together as one commit (D-00b), as the plan directs.
  - Task 1 was the tracer. Its gate passed in autonomous mode: every Task 1 `<verify>` command was re-run on the
    committed tree and exited 0. "Tracer verified end-to-end - expanding."
- **Files modified:** 14 (0 created)

## Accomplishments

- **Red first.**
  - The four paladin-ports tests and the compat tracer failed to compile before the type existed: `E0433` (no
    `BilledUsage`) and `E0599` (no `with_billed_usage` / `billed_usage`).
  - Mutation check on the green tree: passing `None` as the bill's usage in compat `generate` makes the tracer fail
    ("a billed empty completion carries its usage"). The file was then restored and checked with `cmp`.
- **paladin-ports (`llm_port.rs`):**
  - `BilledUsage` comes with rustdoc and an `# Examples` doc test, as do `new` and `with_cost`.
  - Both variants were reshaped. Their doc examples now build through the constructors.
  - Both constructors now set `billed: None`.
  - `with_billed_usage` and `billed_usage` were added, each with a doc example.
  - The `transience` arm is now `EmptyCompletion { .. }`.
  - The transience table gained rows for the billed variants, so both stay `Permanent` with a bill attached.
  - The Display tests now use the constructors.
- **paladin-llm:**
  - `response_guards::bill` was added, with test `bill_attaches_reported_usage_only`.
  - Compat `generate` now maps usage before `classify_completion` and routes the guard's error through
    `bill(err, served_or_request_model, usage)`.
  - `CompatResponse.usage` is now `Option<CompatUsage>` (`#[serde(default)]`), so a body with no usage reaches the
    guard. A successful body without usage still returns the schema-mismatch `ProcessingError`.
- **Remaining sites migrated in the same commit.** No assertion condition changed.
  - `response_guards.rs`: 6 lines. The `Refusal` construction now uses `LlmError::refusal`, `EmptyCompletion(empty_payload(..))`
    now uses `empty_completion`, and `(_)` / `(msg)` patterns became `{ .. }` / `{ detail: msg, .. }`. The
    field-listing `Refusal` pattern at 233 gained `..`.
  - `compat/engine.rs`: 9 lines.
    - `classify_fetch_failure` `EmptyCompletion { .. }`.
    - The retry predicate `| EmptyCompletion { .. }`.
    - "no choices" and the mock "no text" now use `empty_completion`.
    - The `classify_fetch_failure` test `Refusal` uses the constructor.
    - Two `{ detail: msg, .. }` bindings.
    - Two field-listing `Refusal` patterns gained `..`.
  - `anthropic/adapter.rs`: 7 sites.
    - `with_block_census` now edits `detail` in place.
    - The retry predicate.
    - Three `{ detail: msg, .. }` bindings.
    - The mock "no text".
    - Two field-listing `Refusal` patterns gained `..`.
  - `deepseek/adapter.rs`: 12 lines. `annotate_with_usage` now edits `detail` in place (A4-50-26: it still compiles;
    plan 50-27 removes it). The retry predicate, test patterns, bindings and constructions were also migrated.
  - `gemini/adapter.rs`: 13 lines. "no candidates" and the mock "no text" now use `empty_completion`. The retry
    predicate, test patterns, bindings and two assertion-message strings were also migrated.
- **Registration (Task 2), measured and not guessed:**
  - D-27 run: `cargo semver-checks 0.50.0 check-release --package paladin-ports --baseline-version 0.10.1
    --release-type minor`, with all five crate-wide allows commented out. 196 checks, 4 failed.
  - For this plan's items it reported:

    | Item | Lint | Status before this plan |
    |---|---|---|
    | `LlmError::EmptyCompletion` | `enum_tuple_variant_changed_kind` | not allowed: added to `crates/paladin-ports/Cargo.toml` with a comment naming plan 50-26 |
    | `LlmError::EmptyCompletion` | `enum_variant_marked_non_exhaustive` | already allowed (Phase 43): covered by the existing pair, justification extended |
    | `LlmError::Refusal` | none (new since 0.10.1) | -- |
    | `BilledUsage` | none (new item) | -- |

  - The other failures in the run were pre-existing and already allowed. `constructible_struct_adds_field` fired on
    `SubmitRun`, `RunOutcomeRecord`, `LlmResponse.cost`, `LlmResponse.reasoning`, `RunQuery` and
    `CreateRunSchedule`. `enum_unit_variant_changed_kind` fired on `RateLimitExceeded`.
  - The manifest was restored byte for byte, confirmed with `cmp` ("manifest restored byte-for-byte").
  - With the new allow in place, the same 0.10.1 minor run passes: 190 checks, 190 pass.
  - The MIGRATION 9.2 `paladin-ports | LlmError` row was extended ("Extended, Phase 50 plan 50-26"). Its Type cell
    now names the two variants, the four new methods and `BilledUsage`, and Req is `PACE-01, PARITY-05`.
  - Allowlist: one new pair (`paladin-ports`, `enum_tuple_variant_changed_kind`), and the existing
    `enum_variant_marked_non_exhaustive` justification was extended. `make check-migration-allowlist` reports
    set-equal on both directions.
  - MIGRATION 9.8 gained row DR-17, a consumer row on how to build, match and read the new shapes. The PARITY-05
    refusal bullet now ends its pattern in `..`.
  - Muster (D-00i): the PARITY-05 block gained an `EmptyCompletion { detail, .. }` arm, a `billed_usage()` closure
    reading model, `usage.total_tokens` and `cost`, and the two constructors. Everything goes through the existing
    imports only.
  - CHANGELOG: root `### Breaking Changes`, `paladin-ports` `### Breaking Changes`, `paladin-llm` `### Changed`.
  - `make api-surface`: unchanged (4298 items). No `api-surface-update` was needed.

## Task Commits

1. **Tasks 1 and 2 (one commit, D-00b): LlmError carries the usage of billed empty and refused calls; break
   registered**: `329a2922` (feat). `git show --stat` lists `llm_port.rs` and `MIGRATION.md` in that commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `CompatResponse.usage` made optional (`crates/paladin-llm/src/compat/types.rs`, outside the plan's file list)**
- **Found during:** Task 1
- **Issue:** The tracer's "same body with no `usage` object gives `billed_usage() == None`" could not be met. The
  field was required, so such a body failed to parse (`ProcessingError`) before reaching the guard.
- **Fix:** `pub usage: Option<CompatUsage>` with `#[serde(default)]`.
  - A successful response with no usage still returns the same schema-mismatch `ProcessingError`, built in
    `generate` with the same message shape (`missing field \`usage\``) and body excerpt.
  - Behaviour changes on failure paths only: an empty or refused body without `usage` now classifies as
    `EmptyCompletion` / `Refusal` (with no bill) instead of `ProcessingError`.
  - A no-`choices` body without `usage` is now `EmptyCompletion("Response contained no choices")`.
  - The behaviour is described in the paladin-llm CHANGELOG entry. Plan 50-31 owns the 9.1 behaviour row.
- **Commit:** `329a2922`

**2. [Rule 1 - Bug prevention] The two detail annotators keep the bill**
- **Found during:** Task 1
- **Issue:** DeepSeek's `annotate_with_usage` and Anthropic's `with_block_census` rebuilt `EmptyCompletion` from its
  message. Rebuilt the same way on the new shape, they would silently drop any `billed` slot.
- **Fix:** both now edit `detail` in place (`if let LlmError::EmptyCompletion { detail, .. } = &mut err`). Their
  output text is unchanged.
- **Commit:** `329a2922`

### Other deviations

- **CHANGELOG entries in this commit.** The plan put them in plan 50-31, unless `api-surface` changed. The caller
  required them in the same commit as the break, so they were added there: root, paladin-ports and paladin-llm.
  Plan 50-31 extends them with the full billing contract.
- **Three test message strings.** The literals `Err(EmptyCompletion(_))` / `Err(LlmError::EmptyCompletion(_))` in two
  Gemini assertion messages and one Anthropic `panic!` message now read `EmptyCompletion {{ .. }}`. The plan's
  whole-tree `EmptyCompletion(` grep counts string text as well as code. The assertion conditions are unchanged.
- **`#[allow(dead_code)]` on `bill`.** Only the compat buffered path calls it in this plan, so a build without a
  compat preset would warn. `cargo clippy -p paladin-llm --no-default-features --features gemini -- -D warnings` is
  clean. The allow goes away when plan 50-27 bills every adapter.
- **CI's `semver` shape is vacuous at this version.** The plan says the v0.9.0 job "fails on the tuple-to-struct
  change until the lint is allowed". The measurement does not bear that out:
  - `cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0`
    reports `v0.9.0 -> v0.10.1 (major change)` and runs 0 checks (254 skipped). It passed before and after.
  - The registration still follows D-00b, and the 0.10.1 minor run above is what actually gates it.
- **Not billed here, by design:** compat's "Response contained no choices" `EmptyCompletion` (it is not the guard's
  outcome), the compat stream, and every other adapter. All of these are plan 50-27.

## Verification

- **Task 1 verify, all exit 0. Re-run in full on the committed tree for the tracer gate:**
  - `cargo build --workspace --all-targets --all-features`: finished.
  - `cargo test -p paladin-ports`: 259 unit tests passed (4 new); doc tests 203 passed (5 new) / 94 ignored, as before.
  - `cargo test -p paladin-llm --all-features --lib compat_billed_empty_completion_carries_its_usage`: 1 passed.
  - `cargo test -p paladin-llm --all-features --lib`: 1186 passed (1184 + 2 new).
  - `cargo test --doc -p paladin-llm --all-features`: 23 passed.
  - `cargo test -p paladin-battalion --lib llm_`: 30 passed. `cargo test -p paladin-eval --lib`: 56 passed.
  - `make check-consumer-surface`: 1 passed.
- **Task 2 verify, all exit 0:**
  - `make check-migration-allowlist`: set-equal (crate|type pairs, and crate|lint pairs for every crate-wide allow).
  - `make check-consumer-surface`: passed.
  - `make api-surface`: unchanged.
  - `make check-api-examples`: 105 entry points ok.
  - Both greps succeeded. CI's `semver` shape against v0.9.0: exit 0.
- **Acceptance:**
  - `pub struct BilledUsage` 1, `pub fn billed_usage` 1.
  - Whole-tree `EmptyCompletion(` code lines outside `target/`: **0**. With the comment filter removed it is also 0,
    across `crates/`, `src/`, `tests/`, `examples/` and `benches/`.
  - The commit touches none of the seven 50-25 files.
- **Plan-level gates:**
  - `cargo test --workspace --lib --no-fail-fast` (default features): 13 binaries, 5095 passed, 2 failed.
    - The two failures are the ledgered baseline (WINDOWS row 73, recorded in every Phase 50 SUMMARY since 50-03):
      `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set`.
    - Both fail in isolation as well. They drive a `FailingExecutor` that never touches `LlmError`.
    - `--all-features` gives the same two failures (1622 passed in the root crate).
  - `cargo test -p paladin-llm` (default features): 449 passed.
  - `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
  - `cargo fmt --check`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
  - `make clean-code`: exit 0. `make check-gates`: exit 0.
- **`make security`: exit 2. This plan did not cause it.**
  - `cargo-audit` reports exactly RUSTSEC-2026-0367/0368/0369 (`zip 3.0.0`) plus the 9 allowed warnings.
  - This is the open owner decision in `50-VALIDATION.md`. No `Cargo.lock` change.
- **Diff check for `unwrap(` / `expect(` / `panic!(`:** five added occurrences, all inside `#[cfg(test)]` modules:
  - `llm_port.rs` 2984 and 3037 (module starts at 2359).
  - `compat/engine.rs` 3336 and 3344 (module starts at 1436).
  - `anthropic/adapter.rs` 1764 (module starts at 1434).

  The non-test additions use none, and their doc examples use `?`.
- **Manual credential-handling review:**
  - `BilledUsage` holds a model id, `TokenUsage` counts and an `Option<Cost>`. Its derived `Debug` prints those and
    nothing else. No response text and no key reaches it (T-50-26-02).
  - `Refusal.detail` is still built only through `diagnostic_excerpt`, which redacts and then bounds (D-00a).
  - The two annotators changed only how they write `detail`; Anthropic's census still goes through
    `diagnostic_excerpt`.
  - No HTTP client, redirect policy, header or log line changed.
- **Not run:** Docker-bound integration tests, and live vendor calls (no key; the wire request is unchanged).
- **Environment:** disk was the binding constraint.
  - Builds ran with `CARGO_INCREMENTAL=0`.
  - Older-hash duplicate executables in `target/debug/{deps,examples}` (other feature sets, regenerable) were pruned
    three times, recovering about 10 GB at the largest prune.
  - About 6 GB was free at the end.

## Known Stubs

None. The bill is filled on the compat buffered path only. That is this plan's scope (the tracer slice), and plan
50-27 bills every other path.

## Threat Flags

None. Against the plan's register:
- T-50-26-01 (mitigated): the tracer asserts the 63_000-token bill. Other paths are plan 50-27.
- T-50-26-02 (mitigated): numbers, a model id and a `Cost` only.
- T-50-26-03 (mitigated): the break is registered in the same commit, and `#[non_exhaustive]` forces `{ .. }`.
- T-50-26-04 (mitigated): no usage means `None`, never zero. The tracer's second half covers it.

## Flagged assumptions

- A1-50-26 held: struct variants plus constructors follow the `RateLimitExceeded` precedent, and only
  `EmptyCompletion` breaks released code. The measurement confirms it: `Refusal` fired nothing.
- A2-50-26 held: `with_billed_usage` on `RateLimitExceeded` and `NetworkError` returns them unchanged (tested).
- A3-50-26 held: the served model is used when the body names one; an empty `model` falls back to the request model.
- A4-50-26 held: `annotate_with_usage` still compiles and appends the same counts text. Plan 50-27 removes it.

## Self-Check: PASSED

- All 14 modified files exist.
- Commit `329a2922` exists on `claude/laughing-dirac-e0h2ax` and contains both `crates/paladin-ports/src/output/llm_port.rs`
  and `MIGRATION.md`.
- `grep -c 'pub struct BilledUsage' crates/paladin-ports/src/output/llm_port.rs` is 1, and
  `grep -q 'billed_usage' tests/consumer_surface_muster.rs` succeeds.
