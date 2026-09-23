---
phase: 36-rustdoc-zero-warning-bar-examples-currency
fixed_at: 2026-09-18T01:05:54Z
review_path: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 36: Code Review Fix Report

**Fixed at:** 2026-09-18T01:05:54Z
**Source review:** .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (WR-01, WR-02, WR-03, IN-02)
- Fixed: 4
- Skipped: 0

**Out of scope (per orchestrator triage in 36-REVIEW.md, not attempted by this pass):**
- **CR-01** — disposition `no-fix`. The `crates/doc-examples/src/http_service_host.rs`
  router-merge change is EX-55, a deliberately-assigned audit work row (CONTEXT D-19,
  plan 36-09 Task 1, commit `3363d08d`), not a scope violation. The fixer was explicitly
  instructed not to touch this file.
- **IN-01** — follows CR-01's disposition; no action required.

## Fixed Issues

### WR-01: `agent_runtime_middleware.rs` prints a credential-shaped literal without a preceding caveat

**Files modified:** `examples/agent_runtime_middleware.rs`
**Commit:** f0082950
**Applied fix:** Added a one-line `println!("   NOTE: this reason is intentionally NOT
redacted -- see the defect note below")` immediately above the existing
`println!("   reason = {reason}")` line in the fail-run demo (Part 6), so a reader
skimming stdout output sees the caveat before the raw credential-shaped string, not only
in the trailing prose block below it. Verified by running the example: the NOTE line now
appears directly before `reason = Transport error: ... Authorization: Bearer
sk-live-demo0123456789` in program output (exit code 0).

### WR-02: Fixed-delay synchronization is a source of CI flakiness in two examples

**Files modified:** `examples/graceful_shutdown.rs`, `examples/observability_tracing.rs`
**Commit:** 4d225a12
**Applied fix:**
- `graceful_shutdown.rs`: `SlowWorker` now carries a shared `Arc<AtomicUsize>`
  `completions` counter, incremented once each node finishes. `fan_out_graph` returns
  this counter alongside the graph (via a new `FanOutGraph` type alias, added to satisfy
  `clippy::type_complexity`). `drain_run`'s fixed `sleep(Duration::from_millis(30))`
  before `cancel_and_wait` was replaced with a bounded poll (1ms interval, 5s timeout)
  on the fast node's completion count, so shutdown is never triggered before the fast
  node has genuinely finished, regardless of CPU contention. Both call sites (Part 1 and
  Part 2 of `main`) were updated to thread the new counter through.
- `observability_tracing.rs`: `run_demo_graph`'s fixed `sleep(Duration::from_millis(50))`
  before reading the trace recording back was replaced with a bounded poll (5ms interval,
  5s timeout cap) on `RecordingSink::records().len()`, breaking once the count stops
  growing across two consecutive checks. Also fixed a resulting compile error
  (`recording` was moved into `recording_sink` before the later `.records()` call) by
  cloning the `Arc` instead of moving it.

Both programs were rebuilt, `cargo clippy --workspace --all-targets --all-features -- -D
warnings` was clean (via the pre-commit hook), and both were run to completion (exit code
0) with output matching the pre-fix behavior (fast node succeeds, slow node aborted by
shutdown; all 7 trace records captured and read back).

### WR-03: `examples/README.md`'s "Advanced Examples" boilerplate is stale

**Files modified:** `examples/README.md`
**Commit:** 8a9e463c
**Applied fix:**
- Added a disclaimer immediately under the `## Advanced Examples` heading noting these
  snippets are illustrative pseudocode, not compiled/verified like every
  `### [name.rs](name.rs)` example above them, and that `load_config()` /
  `create_llm_adapter()` / `create_fallback_adapter()` are placeholders.
- "Error Handling Patterns": replaced the nonexistent `PaladinBuilder::max_retries` /
  `retry_delay` with the real `retry_attempts` method, and added the missing `.await` on
  `.build()` (verified against `src/application/services/paladin/paladin_builder.rs`:
  `build` is `pub async fn`).
- "Building a Custom Example": replaced the nonexistent builder-style
  `OpenAiAdapter::new().api_key(&api_key).model("gpt-4").build()?` with the real
  `OpenAIAdapter::new(OpenAIConfig::new(api_key))?` construction (verified against
  `crates/paladin-llm/src/openai/adapter.rs`), moved `.model("gpt-4")` onto
  `PaladinBuilder` (the adapter itself has no per-adapter model field), and added the
  missing `.await` on `.build()`.
- "Questions?": replaced the `github.com/your-org/paladin/issues` placeholder with the
  real repository (`github.com/DF3NDR/paladin-dev-env`, per `Cargo.toml`'s `repository`
  field), and dropped the unresolved `discord.gg/paladin (if available)` line (no evidence
  of a real Discord server anywhere else in the project).

Verified: `grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md | wc -l` still returns
62 (no `### [name.rs](name.rs)` section was touched or removed).

### IN-02: `eval_scenarios_demo.rs` prints a since-deleted temp path as copy-pasteable

**Files modified:** `examples/eval_scenarios_demo.rs`
**Commit:** 1b98ab93
**Applied fix:** Added the suggested parenthetical directly after the printed "equivalent
CLI form" command: "(the temp file above is removed when this program exits; point the
glob at a scenario file of your own to actually run this command)". Verified by running
the example (exit code 0); the note now prints immediately below the CLI command line.

## Skipped Issues

None — all four in-scope findings were fixed.

## Verification performed

- `cargo fmt --all` after every edit (clean).
- `cargo build --example <name>` for every modified example (all succeeded).
- `env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY timeout 120 cargo run
  --example <name>` for `agent_runtime_middleware`, `graceful_shutdown`,
  `observability_tracing`, `eval_scenarios_demo` — all four exited 0, offline, no
  provider API key read.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean, both
  standalone (during the WR-02 fix, to resolve a `clippy::type_complexity` finding) and
  via the pre-commit hook on every commit.
- `./scripts/check-api-surface.sh .project/current-exports.txt` — unchanged (3959 items),
  as expected since only `examples/*.rs` and `examples/README.md` changed.
- `grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md | wc -l` — 62, unchanged.

No files outside `examples/*.rs` and `examples/README.md` were modified. No worktree was
used per the orchestrator's instructions for this run (main checkout, branch
`feature/phase-33`); each fix was committed atomically with `git commit` (hooks enforced,
never bypassed).

---

_Fixed: 2026-09-18T01:05:54Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
