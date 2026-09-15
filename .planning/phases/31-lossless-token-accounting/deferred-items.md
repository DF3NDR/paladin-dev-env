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
