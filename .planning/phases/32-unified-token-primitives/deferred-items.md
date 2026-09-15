Deferred Items - Phase 32 (Unified Token Primitives)

Out-of-scope discoveries found during plan execution. Logged per the executor's Scope
Boundary rule - not fixed here because they predate this plan and are unrelated to the
files each discovering task touched.

## Plan 32-03, Task 1

- Pre-existing broken intra-doc link in crates/paladin-memory/src/token_counter/mod.rs:3
  ([HeuristicTokenCounter] fails to resolve under
  RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --no-deps, with or without
  --features content-processing). Confirmed via git stash to exist identically on the
  pre-task base commit (220fc6cd) - introduced in Phase 26 commit 69a56dd6, untouched by
  Task 1's edits to crates/paladin-memory/src/garrison/token_counter.rs. Not caught by CI's
  "Check documentation" job because that job runs cargo doc --workspace --no-deps with
  default features only, and content-processing is not a default feature on any
  workspace-root feature set that enables it transitively for paladin-memory in that job.
  Task 1's own verify step includes this exact cargo doc invocation, so it could not be
  made to exit 0 without editing a file Task 1 does not own; the doc check was instead
  validated narrowly (confirmed the only failure is this pre-existing link, with zero errors
  referencing any name Task 1 removed or renamed). Fix (out of scope for Phase 32): resolve or
  escape the link in token_counter/mod.rs's module doc, e.g. by importing/qualifying
  crate::token_counter::heuristic::HeuristicTokenCounter or converting to a plain code span.

## Plan 32-04, verification

- `cargo test --workspace --all-features --no-fail-fast` reports exactly one failing target:
  `-p paladin-ai --test cli_isolation`, specifically `test_cli_feature_is_not_default`, which
  panics because `--all-features` forces the `cli` feature on for a test asserting it is NOT
  enabled by default. Confirmed unrelated to this plan's files
  (crates/paladin-llm/src/services/commissary.rs,
  src/application/services/paladin/middleware/history.rs, both outside `src/application/cli/`
  and outside any `cli`-feature-gated code path) by reading the panic message and test name --
  it is the same pre-existing `--all-features` vs `cli`-isolation conflict PROJECT.md's Phase 31
  close-out and prior phases already carry as a known, documented, non-blocking item (not
  introduced by Phase 31, unrelated to Phase 32). Fix (out of scope for Phase 32): either scope
  the test to run without `--all-features`, or gate `test_cli_feature_is_not_default` to skip
  when the `cli` feature is externally forced on.
