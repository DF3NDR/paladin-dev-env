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
