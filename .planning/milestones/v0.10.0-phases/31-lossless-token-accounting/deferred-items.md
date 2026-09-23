# Deferred Items — Phase 31

Out-of-scope discoveries logged per the executor's scope-boundary rule (only auto-fix issues
directly caused by the current task's changes).

## Plan 31-01

- **`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under
  `cargo test --workspace --all-features`.** This is a pre-existing, structural conflict
  unrelated to token accounting: the test asserts `#[cfg(feature = "cli")]` is NOT active, but
  `--all-features` always activates the `cli` feature, so the test fails deterministically
  whenever `--all-features` is passed, regardless of any other change in the tree. Confirmed
  pre-existing: the file is untouched by this plan (`git diff --stat <base> -- tests/cli_isolation_test.rs`
  is empty) and last modified in an unrelated historical commit
  (`cefdf2f0 feat: gate CLI module behind optional cli feature flag`). Confirmed the test passes
  cleanly under plain `cargo test --test cli_isolation` (no `--all-features`). Left unfixed per
  the scope boundary — out of scope for this plan.

## Plan 31-05

- **`cargo doc --workspace --no-deps --all-features` emits 16 pre-existing warnings** (9 in
  `paladin-llm`, 7 in `paladin-ai`): private-intra-doc-link warnings (`GeminiAdapter::map_error`,
  `stabilized_fingerprints`, `Self::execute_bounded`, `shadow_validate`,
  `Self::record_engine_failure`, `KNOWN_PROVIDER_NAMES`, `build_reqwest_client`,
  `DEFAULT_SYSTEM_PROMPT`) and unclosed-HTML-tag warnings in `paladin-llm`. None of the
  implicated files (`crates/paladin-llm/src/gemini/adapter.rs`, `src/application/cli/commands/eval.rs`,
  `src/application/services/paladin/paladin_execution_service.rs`,
  `src/application/services/parley/adapter.rs`, `src/application/services/run/worker.rs`,
  `src/config/agent_runtime.rs`, `src/infrastructure/telemetry/otel_sink.rs`, `src/presets/mod.rs`)
  is in this plan's `files_modified` list or touches token-usage/herald code, and none was
  modified by this plan. Confirmed pre-existing per the scope boundary — left unfixed. The
  plan's own verification criterion ("`cargo doc --workspace --no-deps` produces zero warnings")
  is unmet for reasons unrelated to ACCT-04; flagged here rather than silently passed over.
