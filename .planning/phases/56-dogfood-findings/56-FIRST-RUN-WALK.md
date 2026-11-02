# Phase 56: first-run walk on published 0.11.0 (DOGF-04, D-13, D-14)

Keyless half, run by the agent (plan 56-06) on 2026-11-02. Everything up to the first provider call, with
`OPENAI_API_KEY`, `ANTHROPIC_API_KEY` and `DEEPSEEK_API_KEY` unset in every command (`env -u ...`). The keyed
half is the maintainer's (plan 56-08, `56-RUNBOOK-KEYED-WALK.md`). Plan-start SHA: `a39a338dd`.

## Provenance

Scratch crate outside any git repository, no path dependency, no `[patch]`, no workspace.

```
$ WALK=$(mktemp -d "$SCRATCHPAD/paladin-walk.XXXXXX")     # scratch root (session scratchpad, not under the repo)
$ git -C "$WALK" rev-parse --show-toplevel
fatal: not a git repository (or any of the parent directories): .git
rev-parse exit=128
$ rustc -V ; cargo -V
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
$ cd dogfood-walk && cargo metadata --format-version 1 | python3 -I (print name, version, source of package paladin-ai)
paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index
workspace_root = <scratch>/paladin-walk.XXXXXX/dogfood-walk      (the scratch crate itself; no outer workspace)
$ grep -c 'path = ' dogfood-walk/Cargo.toml
0
```

`dogfood-walk/Cargo.toml` `[dependencies]` is exactly `paladin-ai = "0.11.0"` and
`tokio = { version = "1", features = ["full"] }` (from `cargo add`). `cargo tree -p paladin-ai --depth 1` shows
`paladin-ai v0.11.0` with `paladin-ai-core`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`,
`paladin-ports`, `paladin-storage` all `v0.11.0` (registry, no path suffix); `cargo fetch` locked 419 packages.
The other scratch crates (`snippet-as-written`, `quickstart-as-written`, `installation-as-written`) are resolution-only
(`cargo metadata`, no build) and live beside it under the same scratch root; their `Cargo.toml` files carry no
`path = ` entry either. `cargo install` used `--root $WALK/cli` and `--locked` for the working form (the published
lockfile). Published text walked: `registry/src/index.crates.io-*/paladin-ai-0.11.0/README.md`, byte-identical to
`git show v0.11.0:README.md` (`diff` printed nothing).

## Published README vs HEAD

`diff -u` of the published README (what crates.io shows) against HEAD `README.md`, limited to the three sections
walked. Quick Example: identical. Getting Started and Project Status differ only where Phase 54 (FIRST-01/02) fixed
the published text:

```
 [dependencies]
-paladin-ai = "0.5"                      # default features: ["llm-openai"]
+paladin-ai = "0.11.0"                  # default features: ["llm-openai"]
...
-Current version: **0.8.0**.
+Current version: **0.11.0**.
```

(The same diff also shows the "Running agents behind an HTTP API" paragraph reworded by FIRST-01; not on the
first-run path walked here.) HEAD's Getting Started comment still reads `# default features: ["llm-openai"]` while the
root manifest's `default` is `["llm-openai", "llm-anthropic", "llm-deepseek"]` (S4, logged as DF-11). Neither the
published README nor HEAD's has a `cargo add` or `cargo install` line.

## Step table

Rows follow the D-13 order. Every row has its command, exit code and an output excerpt; the last column names the
DF the stumble became, or 'no stumble'. Keys: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY` unset in
every command (`env -u ...`); scratch builds use `CARGO_TARGET_DIR=$WALK/target CARGO_PROFILE_DEV_DEBUG=0
CARGO_INCREMENTAL=0` as a command prefix only.

| # | step | command | exit | output excerpt | stumble → DF |
|---|------|---------|------|----------------|--------------|
| 1 | scratch crate + `cargo add paladin-ai@0.11.0` | `cargo new dogfood-walk && cargo add paladin-ai@0.11.0 && cargo add tokio@1 --features full && cargo fetch` | 0 / 0 / 0 / 0 | `Locking 419 packages to latest Rust 1.97.0 compatible versions`; `cargo metadata` source line in Provenance (`paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index`). The README offers a `[dependencies]` snippet, not a `cargo add` line, on the tag and on HEAD alike; the snippet is a complete way to add the crate, so the absence of a `cargo add` line is not a stumble (the snippet's own content is row 2) | no stumble |
| 2 | README `[dependencies]` snippet as published, resolved as written | `cargo new snippet-as-written`; `[dependencies]` replaced verbatim by the published block (`paladin-ai = "0.5"  # default features: ["llm-openai"]`, `tokio = ...`); `cargo metadata --format-version 1` | 0 | resolved `paladin-ai 0.5.1 registry+https://github.com/rust-lang/crates.io-index` (`Adding paladin-ai v0.5.1 (available: v0.11.0)`); published `Current version: **0.8.0**`. HEAD pins `0.11.0` in both places (FIRST-02), so this is DF-02 as published. The `default features` comment names one of three defaults (HEAD too) | DF-10 (duplicate of DF-02), DF-11 |
| 3 | README Quick Example (`MockLlmAdapter`, no key) | `cargo run` with the scratch prefix (block copied verbatim from the published README into `src/main.rs`) | 0 | `Finished dev profile [unoptimized] target(s) in 1m 22s`, then `Hello from Paladin!` | no stumble |
| 4 | `basic_paladin` from the published package source | `cp ~/.cargo/registry/src/*/paladin-ai-0.11.0/examples/basic_paladin.rs dogfood-walk/examples/`; `cargo run --example basic_paladin` (scratch prefix). Then `cargo add paladin-ports@0.11.0` and the same command | 101, then 0 | error[E0433]: cannot find module or crate `paladin_ports` in this scope, at `examples/basic_paladin.rs:22:5`; after adding `paladin-ports`, the example prints the Herald result and `✨ Example completed successfully!`. No doc gives a crates.io user a command for it (the Quickstart's route is `git clone`) | DF-12 |
| 5 | Quickstart with no key | tag Quickstart `[dependencies]` resolved as written (`cargo metadata`, in `quickstart-as-written`): `paladin-ai 0.10.1`, `paladin-llm 0.10.1`; then `cargo add paladin-ports@0.11.0` and `cargo add paladin-llm@0.11.0 --features openai`, tag `main` in `src/bin/quickstart.rs`, `env -u OPENAI_API_KEY cargo run --bin quickstart` | 0 (resolution), 1 (run) | the tag Quickstart pins `0.10.0` (resolves to 0.10.1); the walk used `0.11.0`. Run: `Error: "OPENAI_API_KEY environment variable not set"`. The error names the variable, so the missing-key message is not a stumble | DF-13 (duplicate of DF-02) for the version pin; the key error is no stumble |
| 5b | lead L1: installation page's `[dependencies]` block | `cargo metadata` on the block of `docs/src/getting-started/installation.md` (HEAD line 57; versions as HEAD, `0.11.0`) in `installation-as-written`; then the same with `features = ["openai"]` | 101, then 0 | package `installation-as-written` depends on `paladin-llm` with feature `llm-openai` but `paladin-llm` does not have that feature (help: available features: anthropic, deepseek, default, gemini, grok, kimi, mock, ollama, openai, ...); with `openai`: resolves, all seven `paladin-*` at `0.11.0`. The tag's block fails the same way | DF-14 |
| 6 | free disk: delete the scratch build | `rm -rf $WALK/target`; `df -BM --output=avail /` | 0 | 4327 MB → 5476 MB free | no stumble |
| 7 | lead L2: `cargo install` of `paladin-cli` | (a) `cargo install paladin-ai --version 0.11.0 --bin paladin-cli --root $WALK/cli`; (b) `CARGO_TARGET_DIR=$WALK/install-target cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root $WALK/cli`, then `rm -rf $WALK/install-target` | (a) 101, (b) 0 | (a) target `paladin-cli` in package `paladin-ai` requires the features: `cli` (Consider enabling them by passing, e.g., `--features="cli"`). (b) Finished `release` profile [optimized] target(s) in 5m 12s, Installed package `paladin-ai v0.11.0` (executable `paladin-cli`). A1 proven | DF-15 |
| 8 | `paladin-cli --version` and `--help` | `$WALK/cli/bin/paladin-cli --version`; `... --help` | 0 / 0 | `paladin 0.11.0`; `--help` lists 13 subcommands (agent, battalion, arsenal, maneuver, onboarding, setup-check, features, muster, eval, graph, run, treasury, council). The version string is `paladin`, not `paladin-cli` (the cause of DF-17); the docs show no `--version` sample | DF-17 (cause); no separate stumble |
| 9 | `setup-check`, keyless | in an empty `$WALK/run`, with the three keys, `REDIS_URL` and `QDRANT_URL` unset: `paladin-cli setup-check`, `... --verbose`, `... --quiet` | 2 / 2 / 2 | see Keyless observations: bare `Service:` and `LLM Provider:` headings with no rows; `Passed: 2`, `Warnings: 1`; section order differs run to run (five runs); exit 2 is the documented warning exit | DF-16 |
| 10 | `onboarding` with no TTY | `timeout 20 paladin-cli onboarding < /dev/null` in `$WALK/run` | 2 | welcome banner, then `Error: User input error: IO error: not a terminal`. It fails at once with a clear message and does not hang; no file written (`find . -mindepth 1` printed nothing) | no stumble |
| 11 | the docs' command as printed (`paladin onboarding`, `paladin setup-check`) | `env -i PATH=$WALK/cli/bin /bin/bash -c 'paladin onboarding; paladin setup-check; command -v paladin-cli'` | 127 / 127 | `paladin: command not found` twice; `command -v paladin-cli` finds the installed binary. Six CLI appendix pages and the binary's own messages name `paladin` | DF-17 |

## Keyless observations

Quoted verbatim from the runs above (provider keys unset).

**Quickstart missing-key error** (step 5; tag `main` with `OpenAIAdapter::from_env()?`, no `OPENAI_API_KEY`):

```
Error: "OPENAI_API_KEY environment variable not set"
```

It names the variable. The message is Debug-formatted (quoted), which is how `fn main() -> Result<(), Box<dyn Error>>`
prints a `String` error; not logged.

**`setup-check`, default** (step 9, exit 2; run in an empty directory, the three provider keys, `REDIS_URL` and
`QDRANT_URL` unset; this run happened to print `Service`, `LLM Provider`, `Environment`, `System`):

```
┌═══════════════════════┐
│ Paladin Setup Check │
└═══════════════════════┘



Service:

LLM Provider:

Environment:
  ⚠ .env file 
⚠   Not found - run 'paladin onboarding' to create

System:
  ✓ Paladin CLI v0.11.0 
  ✓ Rust Toolchain (rustc 1.97.0 (2d8144b78 2026-07-07)) 

┌═══════════┐
│ Summary │
└═══════════┘
  Passed: 2
  Warnings: 1

⚠ Setup check completed with warnings

┌────────────────────────────────────────────────────────────┐
│ Next Steps:                                                │
│ 1. Run 'paladin onboarding' to configure missing providers │
│ 2. Verify environment variables in your .env file          │
│ 3. Run 'paladin setup-check' again to verify               │
└────────────────────────────────────────────────────────────┘
exit=2
```

**`setup-check --verbose`** (step 9, exit 2; section rows only, banner, summary and Next Steps box as above;
Summary adds `Skipped: 5`):

```
LLM Provider:
  ○ OpenAI 
ℹ   OPENAI_API_KEY not configured
  ○ Anthropic 
ℹ   ANTHROPIC_API_KEY not configured
  ○ DeepSeek 
ℹ   DEEPSEEK_API_KEY not configured

Service:
  ○ Redis 
ℹ   REDIS_URL not configured
  ○ Qdrant 
ℹ   QDRANT_URL not configured

Environment:
  ⚠ .env file 
⚠   Not found - run 'paladin onboarding' to create

System:
  ✓ Paladin CLI v0.11.0 
ℹ   All checks passed
  ✓ Rust Toolchain (rustc 1.97.0 (2d8144b78 2026-07-07)) 
ℹ   All checks passed
```

`--quiet` printed the same rows as the default, with `System` first and `LLM Provider` last (exit 2). Section order
across five runs of the same command, headings only: `Service LLM Environment System`; `LLM Service Environment System`
(`--verbose`); `System Service Environment LLM` (`--quiet`); `System LLM Service Environment`;
`LLM Environment System Service`. No key was reported present when none was set. The exit code (2) agrees with the
rows (one warning) and with the documented exit-code table.

**`onboarding`, no TTY** (step 10, `< /dev/null`, `timeout 20`; the box-drawing banner lines elided):

```
 Welcome 
 Welcome to Paladin! 🛡️ 
✓ Let's get you set up with a multi-agent orchestration framework.

This wizard will help you:
  • Configure LLM provider API keys
  • Set up your environment file
  • Validate API connectivity
  • Generate sample configurations


Error: User input error: IO error: not a terminal

exit=2
```

No `.env` and no YAML was written (the directory was empty afterwards). It neither hangs nor prompts; this replaces
research assumption A4 ("fails or hangs"): it fails at once and says why.

**Lead outcomes.** L1 reproduced on the published `0.11.0` crates (DF-14). L2 reproduced (DF-15) and the working form
`cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli` is proven (assumption A1).

## Disk ledger

Plan PD-06-2. The scratch root no longer exists (`test ! -e "$WALK"` printed `scratch removed`). Scratch builds used `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0` and a throwaway `CARGO_TARGET_DIR`;
nothing was exported into a shell that builds the workspace, and no workspace `cargo build` or `cargo test` ran.

| point | free on `/` |
|-------|-------------|
| plan start (before any build) | 5513 MB |
| after freeing `target/debug/examples` and `target/debug/incremental` (both were already empty, 4 KB each, so nothing to free) | 5513 MB |
| after the step-3 scratch build (`$WALK/target` present) | 4338 MB |
| after the step-4 and step-5 builds (same `$WALK/target`, incremental) | 4327 MB |
| step 6: after `rm -rf $WALK/target` | 5476 MB |
| step 7(a): resolve, download and fail (no build) | 5476 MB |
| step 7(b): during the release install (`install-target` present) | 4316 MB at the end of the build |
| after `rm -rf $WALK/install-target` | 5455 MB |
| end: after `rm -rf $WALK` (scratch root, every scratch crate, `cli/` install root) | 5476 MB |

No step hit `No space left on device`; no step is marked 'not run: disk'.

## Findings-log check

`./scripts/check-findings-log.sh` after the last entry of this plan (exit 1, expected; exit status is not asserted
because contained docs entries stay `open` until plan 56-07 and DF-07/DF-08 are Stage C):

```
DF-07: OPEN   DF-08: OPEN   DF-11: OPEN   DF-12: OPEN   DF-14: OPEN   DF-15: OPEN   DF-17: OPEN
parsed 17 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

DF-10 and DF-13 parse as `duplicate of DF-02`; DF-16 parses as `out (re-homed: ...)`. `git diff --numstat` for the log
over this plan shows 0 deleted lines.

## Contained fixes

Plan 56-07 (docs only, plan-start SHA `15392e9aa`), 2026-11-02. Proofs that need crates.io run in a `mktemp -d`
scratch crate outside the repo and are resolution-only (`cargo metadata`, no compile); the scratch is deleted
afterwards. Columns: `DF | file(s) | change | proof command | result`.

| DF | file(s) | change | proof command | result |
|----|---------|--------|---------------|--------|
| DF-14 (L1) | `docs/src/getting-started/installation.md` | `paladin-llm` line: `features = ["llm-openai"]` becomes `features = ["openai"]`. No other `paladin-llm` dependency line in `docs/src` or `README.md` used an `llm-` feature (`docs/src/getting-started/quickstart.md:26` and `docs/src/api-reference/crate-map.md:230` already say `openai`) | scratch crate whose `[dependencies]` is the corrected block extracted from the page: `cargo metadata --format-version 1 > /dev/null`; then the same block with `features = ["llm-openai"]` | corrected block: exit 0, `Locking 225 packages`, `paladin-ai-core`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-ports`, `paladin-storage` all `0.11.0`. Old block: exit 101, `package \`installation-old\` depends on \`paladin-llm\` with feature \`llm-openai\` but \`paladin-llm\` does not have that feature.` |
| DF-15 (L2) | `docs/src/getting-started/installation.md`, `docs/src/appendix/cli-onboarding.md` | adds `cargo install paladin-ai --features cli --locked --bin paladin-cli` in a fenced `bash` block, with a sentence that the `cli` feature is required and `--locked` uses the published lockfile. The line has no `--version`, so it cannot go stale (PD-07-1) | plan 56-06 step 7(b) ran the same line with `--version 0.11.0 --root <scratch>`; step 7(a) without `--features cli` failed; `grep -rn 'cargo install paladin-ai --features cli --locked --bin paladin-cli' docs/src/getting-started/installation.md docs/src/appendix/cli-onboarding.md` | 7(b) exit 0 (`Installed package paladin-ai v0.11.0 (executable paladin-cli)`, A1 proven); 7(a) exit 101 (`requires the features: cli`). grep prints 2 lines (`installation.md:141`, `cli-onboarding.md:22`). The `--version`-less form was not itself re-run here: it differs from the proven run only by resolving the latest published `paladin-ai`, which is `0.11.0` |
| DF-11 (S4) | `README.md`, `docs/src/api-reference/crate-map.md` | the comment after `paladin-ai = "0.11.0"` now reads `# default features: ["llm-openai", "llm-anthropic", "llm-deepseek"]` | `python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb'))['features']['default'])"` and `grep -n 'default features' README.md docs/src/api-reference/crate-map.md` | `['llm-openai', 'llm-anthropic', 'llm-deepseek']`; both comments list those three names in that order. `make check-version-strings` exit 0; `make check-doc-config` exit 0 (161 YAML blocks, 0 failed) |
