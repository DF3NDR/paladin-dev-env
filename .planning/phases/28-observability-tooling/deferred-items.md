# Deferred items — Phase 28 (observability-tooling)

Out-of-scope discoveries logged per the executor's scope-boundary rule
(only auto-fix issues directly caused by the current task's own changes).

## 28-09: `paladin_builder.rs`'s test module is missing `llm-deepseek`/`llm-anthropic` feature gates

**Found during:** 28-09 Task 2, while choosing the exact CI command for the
new `otel-feature` job in `.github/workflows/feature-flags.yml`.

**Symptom:** `cargo test -p paladin-ai --lib --no-default-features` (no
`otel` feature involved at all) fails to compile with:

```
error[E0433]: cannot find `deepseek` in `paladin_llm`
  --> src/application/services/paladin/paladin_builder.rs:1530:35
error[E0433]: cannot find `deepseek` in `paladin_llm`
  --> src/application/services/paladin/paladin_builder.rs:1535:31
```

`paladin_builder.rs`'s own `#[cfg(test)]` module references
`paladin_llm::deepseek::{DeepSeekConfig, DeepSeekAdapter}` (and, per
`tests/unit/llm/anthropic_adapter_test.rs` /
`tests/unit/llm/deepseek_adapter_test.rs` /
`tests/integration/provider_switching_test.rs`, the same gap recurs for
`anthropic`) with no `#[cfg(feature = "llm-deepseek")]` (or
`llm-anthropic`) gate, so a `--no-default-features` build of just the
`paladin-ai` package in isolation (`-p paladin-ai`, not `--workspace`)
fails outside those features' presence.

**Why it doesn't already break the existing 14-leg feature-flags matrix:**
the `no-default-features` matrix leg's own `Test` step is
`cargo test --workspace --lib ${{ matrix.flags }}` -- `--workspace`, not
`-p paladin-ai`. Cargo's feature resolver unifies features across every
workspace member being built in a `--workspace` invocation, and some
other member's own dependency edge apparently keeps `paladin_llm`'s
`deepseek`/`anthropic` features enabled even when paladin-ai's own
`--no-default-features` flag is set. Selecting `-p paladin-ai` alone (no
sibling members built) removes that incidental unification and exposes
the gap. Verified empirically (28-09): `cargo test --workspace --lib
--no-default-features` passes (943 `paladin` lib tests, including this
plan's own `otel_sink` tests when `--features otel` is added); `cargo test
-p paladin-ai --lib --no-default-features` fails to compile.

**Action taken:** the new `otel-feature` CI job (28-09 Task 2) uses
`cargo test --workspace --no-default-features --features otel ...` rather
than `-p paladin-ai`, matching the existing matrix legs' own convention and
avoiding the gap entirely -- not a fix, a routing decision that doesn't
touch the pre-existing bug.

**Not fixed here:** out of 28-09's scope (unrelated file, unrelated
feature). A future plan touching `paladin_builder.rs`'s test module,
`tests/unit/llm/anthropic_adapter_test.rs`, or
`tests/unit/llm/deepseek_adapter_test.rs` should add the missing
`#[cfg(feature = "llm-deepseek")]` / `#[cfg(feature = "llm-anthropic")]`
gates so `-p paladin-ai --no-default-features` (isolated from the rest of
the workspace) also compiles cleanly.
