//! The `evals` custom test harness target (D-32, PRD 07 OBS-FR-13).
//!
//! `harness = false` -- this is NOT compiled against the built-in `libtest`
//! harness; `paladin_eval::eval_scenarios!` expands to its own `fn main()`
//! that globs the pattern below at runtime and hands one
//! `libtest_mimic::Trial` per `(scenario file, case)` pair to `paladin-eval`'s
//! own custom harness, so `cargo test --test evals <filter>` filters exactly
//! like any other Rust test target.
//!
//! `evals/` does not exist yet in this workspace (plan 28-16 populates it with
//! the E2E-1/2/3 dogfood scenarios) -- the glob below matching nothing is a
//! valid, green `test result: ok. 0 passed` run, not a failure.

paladin_eval::eval_scenarios!("evals/**/*.eval.yaml");
